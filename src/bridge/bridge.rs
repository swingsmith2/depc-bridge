use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use log::{error, info};

use anyhow::Result;
use tokio::{
    join,
    sync::mpsc::{channel, Receiver, Sender},
    time::{sleep, Duration},
};

use crate::db;
use crate::depc::{
    extract_string_from_script_hex, Address as DePCAddress, Amount as DePCAmount,
    Client as DePCClient, TxID as DePCTxID,
};

pub trait ContractClient {
    type Error: std::fmt::Display;
    type Address: Into<String> + From<String> + Copy;
    type Amount: Into<u64> + From<u64> + Copy;
    type TxID: Into<String> + From<String> + Copy;

    fn send(&self, address: Self::Address, amount: Self::Amount)
        -> Result<Self::TxID, Self::Error>;

    fn load_unfinished_withdrawals(
        &self,
    ) -> Result<Vec<(Self::TxID, Self::Address, Self::Amount)>, Self::Error>;
}

pub struct WithdrawInfo {
    txid: DePCTxID,
    address: DePCAddress,
    amount: DePCAmount,
}

pub struct DepositInfo<C: ContractClient> {
    address: C::Address,
    amount: C::Amount,
}

pub struct Bridge<C>
where
    C: ContractClient,
{
    exit_sig: Arc<Mutex<bool>>,
    conn: db::Conn,
    depc_client: DePCClient,
    depc_owner_address: DePCAddress,
    contract_client: C,
    tx_deposit: Sender<DepositInfo<C>>,
    rx_deposit: Receiver<DepositInfo<C>>,
    tx_withdraw: Sender<WithdrawInfo>,
    rx_withdraw: Receiver<WithdrawInfo>,
}

impl<C> Bridge<C>
where
    C: ContractClient,
{
    pub fn new(
        conn: db::Conn,
        depc_client: DePCClient,
        depc_owner_address: DePCAddress,
        contract_client: C,
    ) -> Self {
        let (tx_deposit, rx_deposit) = channel::<DepositInfo<C>>(1);
        let (tx_withdraw, rx_withdraw) = channel::<WithdrawInfo>(1);
        Bridge::<C> {
            exit_sig: Arc::new(Mutex::new(false)),
            conn,
            depc_client,
            depc_owner_address,
            contract_client,
            tx_deposit,
            rx_deposit,
            tx_withdraw,
            rx_withdraw,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        // let withdraw_making_handler = tokio::spawn(Bridge::<C>::withdraw_making(Arc::clone(&bridge)));
        // withdraw_making_handler.await;
        // Ok(())

        let withdraw_making_task = tokio::spawn(withdraw_making(
            Arc::clone(&self.exit_sig),
            self.rx_withdraw,
            self.depc_owner_address,
            self.depc_client,
        ));

        join!(withdraw_making_task);

        todo!("complete this method");
    }
}

pub async fn withdraw_making(
    exit_sig: Arc<Mutex<bool>>,
    mut rx_withdraw: Receiver<WithdrawInfo>,
    depc_owner_address: DePCAddress,
    depc_client: DePCClient,
) -> Result<()> {
    loop {
        {
            let exit = exit_sig.lock().unwrap();
            if *exit {
                break;
            }
        }
        if let Some(withdraw) = rx_withdraw.recv().await {
            depc_client.transfer(&depc_owner_address, &withdraw.address, withdraw.amount)?;
        }
        sleep(Duration::from_secs(1)).await;
    }
    Ok(())
}

pub async fn deposit_making<C>(
    exit_sig: Arc<Mutex<bool>>,
    mut rx_deposit: Receiver<DepositInfo<C>>,
    contract_client: C,
    conn: db::Conn,
) -> Result<()>
where
    C: ContractClient,
{
    loop {
        {
            let exit = exit_sig.lock().unwrap();
            if *exit {
                break;
            }
        }
        if let Some(deposit) = rx_deposit.recv().await {
            match contract_client.send(deposit.address, deposit.amount) {
                Ok(txid) => {
                    // update database
                    conn.confirm_deposit(&(txid).into(), get_curr_timestamp(), "")?;
                }
                Err(e) => {
                    error!(
                        "cannot send transaction to solana to make deposit, reason: {}",
                        e
                    );
                }
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
    Ok(())
}

pub async fn run_depc_syncing<C>(
    exit_sig: Arc<Mutex<bool>>,
    conn: db::Conn,
    depc_client: DePCClient,
    depc_owner_address: DePCAddress,
    tx_deposit: Sender<DepositInfo<C>>,
) -> Result<()>
where
    C: ContractClient,
{
    let mut sync_height = if let Some(height) = conn.query_best_height() {
        height + 1
    } else {
        0
    };
    conn.begin_transaction()?;

    loop {
        {
            let exit = exit_sig.lock().unwrap();
            if *exit {
                break;
            }
        }
        let chain_height = depc_client.get_height()?;
        if sync_height > chain_height {
            // there is no more block left to sync, wait for 5 seconds...
            sleep(Duration::from_secs(5)).await;
            continue;
        }
        info!(
            "syncing from height {sync_height} to chain height {chain_height}, distance {}",
            chain_height - sync_height
        );

        // block
        let block_hash = depc_client.get_block_hash(sync_height)?;
        let block = depc_client.get_block(&block_hash)?;
        assert_eq!(block.height, sync_height);
        conn.add_block(&block.hash, sync_height, &block.miner, block.time)?;

        if sync_height > 0 {
            // transactions
            for txid in block.tx.iter() {
                let transaction = depc_client.get_transaction(txid)?;
                let mut deposit_info = None;
                assert_eq!(transaction.txid, *txid);
                conn.add_transaction(&block_hash, txid)?;
                for txin in transaction.vin.iter() {
                    if !txin.is_coinbase() {
                        // TODO maybe we need to check the validity of the txin?
                        conn.mark_coin_to_spent(
                            &txin.txid.clone().unwrap(),
                            txin.vout.unwrap(),
                            txid,
                            sync_height,
                        )?;
                    }
                }
                let mut amount = 0u64;
                for txout in transaction.vout.iter() {
                    // save the txout anyway
                    if let Some(address) = txout.get_address() {
                        conn.add_coin(
                            txid,
                            txout.n,
                            txout.value64,
                            &address,
                            &txout.script_pubkey.hex,
                        )?;
                        // check the coin is mine?
                        if address == depc_owner_address {
                            amount += txout.value64;
                        }
                    } else {
                        // maybe it is the script with erc20 address
                        if let Ok(str) = extract_string_from_script_hex(&txout.script_pubkey.hex) {
                            deposit_info = Some(str);
                        }
                    }
                }
                if deposit_info.is_some() && amount > 0 {
                    let to_erc20_address_str = deposit_info.unwrap();
                    conn.make_deposit(txid, &to_erc20_address_str, amount, block.time)
                        .unwrap();
                    // we need to put this deposit to the consumer thread to make it happend on solana network
                    tx_deposit
                        .send(DepositInfo {
                            address: to_erc20_address_str.into(),
                            amount: amount.into(),
                        })
                        .await
                        .unwrap();
                }
            }
        }

        sync_height += 1;
    }
    conn.commit_transaction()?;

    Ok(())
}

pub async fn run_sol_syncing<C>(
    exit_sig: Arc<Mutex<bool>>,
    contract_client: C,
    conn: db::Conn,
) -> Result<()>
where
    C: ContractClient,
{
    loop {
        {
            let exit = exit_sig.lock().unwrap();
            if *exit {
                break;
            }
            match contract_client.load_unfinished_withdrawals() {
                Ok(withdrawals) => {
                    for (txid, address, amount) in withdrawals.iter() {
                        // current timestamp we should retrieve
                        let timestamp = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        // we need to save the withdrawal into database
                        conn.make_withdraw(
                            &(*txid).into(),
                            timestamp,
                            &(*address).into(),
                            (*amount).into(),
                        )
                        .unwrap();
                    }
                }
                Err(e) => {
                    error!("cannot load unfinished withdrawals, reason: {}", e);
                }
            }
        }
    }
    Ok(())
}

fn get_curr_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
