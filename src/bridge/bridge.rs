use std::fmt::Debug;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use log::{error, info};

use anyhow::{bail, Result};
use tokio::{
    sync::mpsc::{channel, Receiver, Sender},
    time::{sleep, Duration},
};

use crate::db;
use crate::depc::{
    extract_string_from_script_hex, Address as DePCAddress, Amount as DePCAmount,
    Client as DePCClient, TxID as DePCTxID,
};

pub trait TokenClient {
    type Error: std::fmt::Display + std::fmt::Debug;
    type Address: ToString + FromStr + Clone + Send;
    type Amount: Into<u64> + From<u64> + Clone + Send;
    type TxID: ToString + FromStr + Clone + Send;

    fn send(
        &self,
        recipient_address: &Self::Address,
        amount: Self::Amount,
    ) -> Result<Self::TxID, Self::Error>;

    fn load_unfinished_withdrawals(
        &self,
    ) -> Result<Vec<(Self::TxID, Self::Address, Self::Amount)>, Self::Error>;
}

pub struct WithdrawInfo {
    txid: DePCTxID,
    address: DePCAddress,
    amount: DePCAmount,
}

pub struct DepositInfo<Address, Amount> {
    sender_address: Address,
    recipient_address: Address,
    amount: Amount,
}

pub struct Bridge<C>
where
    C: TokenClient,
{
    exit_sig: Arc<Mutex<bool>>,
    conn: db::Conn,
    depc_client: DePCClient,
    depc_owner_address: DePCAddress,
    contract_client: C,
    tx_deposit: Sender<DepositInfo<C::Address, C::Amount>>,
    rx_deposit: Receiver<DepositInfo<C::Address, C::Amount>>,
    tx_withdraw: Sender<WithdrawInfo>,
    rx_withdraw: Receiver<WithdrawInfo>,
}

impl<C> Bridge<C>
where
    C: TokenClient + Clone + 'static + Send,
{
    pub fn new(
        conn: db::Conn,
        depc_client: DePCClient,
        depc_owner_address: DePCAddress,
        contract_client: C,
    ) -> Self {
        let (tx_deposit, rx_deposit) = channel::<DepositInfo<C::Address, C::Amount>>(1);
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

    pub async fn run(self) -> Result<()> {
        let mut tasks = vec![];

        let withdraw_making_task = tokio::spawn(withdraw_making(
            Arc::clone(&self.exit_sig),
            self.rx_withdraw,
            self.depc_owner_address.clone(),
            self.depc_client.clone(),
        ));
        tasks.push(withdraw_making_task);

        let deposit_making_task = tokio::spawn(deposit_making(
            Arc::clone(&self.exit_sig),
            self.rx_deposit,
            self.contract_client.clone(),
            self.conn.clone(),
        ));
        tasks.push(deposit_making_task);

        let depc_syncing_task = tokio::spawn(run_depc_syncing::<C>(
            Arc::clone(&self.exit_sig),
            self.conn.clone(),
            self.depc_client,
            self.depc_owner_address,
            self.tx_deposit,
        ));
        tasks.push(depc_syncing_task);

        let sol_syncing_task = tokio::spawn(run_sol_syncing::<C>(
            Arc::clone(&self.exit_sig),
            self.contract_client,
            self.conn,
        ));
        tasks.push(sol_syncing_task);

        futures::future::join_all(tasks).await;
        Ok(())
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
    mut rx_deposit: Receiver<DepositInfo<C::Address, C::Amount>>,
    contract_client: C,
    conn: db::Conn,
) -> Result<()>
where
    C: TokenClient,
{
    loop {
        {
            let exit = exit_sig.lock().unwrap();
            if *exit {
                break;
            }
        }
        if let Some(deposit) = rx_deposit.recv().await {
            match contract_client.send(&deposit.recipient_address, deposit.amount) {
                Ok(txid) => {
                    // update database
                    conn.confirm_deposit(&txid.to_string(), get_curr_timestamp(), "")?;
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
    tx_deposit: Sender<DepositInfo<C::Address, C::Amount>>,
) -> Result<()>
where
    C: TokenClient,
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
                    let sender_address = C::Address::from_str("TODO the sender address should be retrieved from config or command-line arguments").unwrap_or_else(|_| {
                        panic!("invalid address");
                    });
                    let recipient_address = C::Address::from_str(&to_erc20_address_str)
                        .unwrap_or_else(|_| {
                            panic!("invalid address");
                        });
                    tx_deposit
                        .send(DepositInfo::<C::Address, C::Amount> {
                            sender_address,
                            recipient_address,
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
    C: TokenClient,
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
                            &txid.to_string(),
                            timestamp,
                            &address.to_string(),
                            (amount.clone()).into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_curr_timestamp() {
        let timestamp = get_curr_timestamp();
        assert!(timestamp > 0);
    }
}