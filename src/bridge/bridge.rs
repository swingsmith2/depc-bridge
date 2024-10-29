use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use log::{error, info};

use anyhow::Result;
use solana_sdk::signature::Signature;
use tokio::{
    sync::mpsc::{channel, Receiver, Sender},
    time::{sleep, Duration},
};

use crate::db;
use crate::depc::{extract_string_from_script_hex, Address as DePCAddress, Amount as DePCAmount, Client as DePCClient, TxID as DePCTxID, Address};
use crate::solana::{SolanaClient as SolanaClient, TokenClient};
const DEPOSIT_THRESHOLD: u64 = 1000;
const WITHDRAW_THRESHOLD: u64 = 1000;
pub struct WithdrawInfo {
    sender_address: DePCAddress,
    recipient_address: DePCAddress,
    amount: u64,
}

pub struct DepositInfo<Address, Amount> {
    sender_address: Address,
    recipient_address: Address,
    amount: Amount,
}
pub struct DepcScriptData<Address> {
    pub recipient: Address,
    pub signature: Signature
}

pub struct Bridge<C>
where
    C: TokenClient,
{
    exit_sig: Arc<Mutex<bool>>,
    conn: db::Conn,
    depc_client: DePCClient,
    depc_owner_address: DePCAddress,
    solana_owner_address: String,
    contract_client: C,
    tx_deposit: Sender<DepositInfo<C::Address, C::Amount>>,
    rx_deposit: Receiver<DepositInfo<C::Address, C::Amount>>,
    tx_withdraw: Sender<WithdrawInfo>,
    rx_withdraw: Receiver<WithdrawInfo>,
}

impl<C> Bridge<C>
where
    C: TokenClient + 'static + Send + Clone,
{
    pub fn new(
        conn: db::Conn,
        depc_client: DePCClient,
        depc_owner_address: DePCAddress,
        solana_owner_address: String,
        contract_client: C,
    ) -> Self {
        let (tx_deposit, rx_deposit) = channel::<DepositInfo<C::Address, C::Amount>>(1);
        let (tx_withdraw, rx_withdraw) = channel::<WithdrawInfo>(1);
        Bridge::<C> {
            exit_sig: Arc::new(Mutex::new(false)),
            conn,
            depc_client,
            depc_owner_address,
            solana_owner_address,
            contract_client,
            tx_deposit,
            rx_deposit,
            tx_withdraw,
            rx_withdraw,
        }
    }

    pub async fn run(self) -> Result<()> {
        let mut tasks = vec![];

        let withdraw_making_task = tokio::spawn(withdraw_processing(
            Arc::clone(&self.exit_sig),
            self.rx_withdraw,
            self.depc_owner_address.clone(),
            self.depc_client.clone(),
        ));
        tasks.push(withdraw_making_task);

        let deposit_making_task = tokio::spawn(deposit_processing(
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
            self.contract_client,
            self.depc_owner_address,
            self.solana_owner_address,
            self.tx_deposit,
            self.tx_withdraw,
        ));
        tasks.push(depc_syncing_task);

        futures::future::join_all(tasks).await;
        Ok(())
    }
}

pub async fn withdraw_processing(
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
            depc_client.transfer(&depc_owner_address, &withdraw.recipient_address, withdraw.amount)?;
        }
        sleep(Duration::from_secs(1)).await;
    }
    Ok(())
}

pub async fn deposit_processing<C>(
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
            match contract_client.send_token(&deposit.recipient_address, deposit.amount) {
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
    local_db: db::Conn,
    depc_client: DePCClient,
    contract_client: C,
    depc_owner_address: DePCAddress,
    solana_owner_address: String,
    tx_deposit: Sender<DepositInfo<C::Address, C::Amount>>,
    tx_withdraw: Sender<WithdrawInfo>, // TODO matthew: deliver the withdrawal to this channel
) -> Result<()>
where
    C: TokenClient + Send + 'static,
    C::Error: Send + 'static,
{
    let mut sync_height = if let Some(height) = local_db.query_best_height() {
        height + 1
    } else {
        0
    };
    local_db.begin_transaction()?;

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
        local_db.add_block(&block.hash, sync_height, &block.miner, block.time)?;

        if sync_height > 0 {
            // transactions
            for txid in block.tx.iter() {
                let transaction = depc_client.get_transaction(txid)?;
                                                                    // information should be
                                                                    // extracted from txouts
                assert_eq!(transaction.txid, *txid);
                local_db.add_transaction(&block_hash, txid)?;
                for txin in transaction.vin.iter() {
                    if !txin.is_coinbase() {
                        // TODO maybe we need to check the validity of the txin?
                        local_db.mark_coin_to_spent(
                            &txin.txid.clone().unwrap(),
                            txin.vout.unwrap(),
                            txid,
                            sync_height,
                        )?;
                    }
                }
                for txout in transaction.vout.iter() {
                    // save the txout anyway
                    if let Some(address) = txout.get_address() {
                        local_db.add_coin(
                            txid,
                            txout.n,
                            txout.value64,
                            &address,
                            &txout.script_pubkey.hex,
                        )?;
                        // is our address,start processing
                        if address == depc_owner_address{
                            if let Ok(script_data) = extract_string_from_script_hex(&txout.script_pubkey.hex) {
                                if txout.value64 > DEPOSIT_THRESHOLD && script_data.recipient != ""{  //deposit
                                    local_db
                                        .save_deposit(txid, &script_data.recipient, txout.value64, block.time)
                                        .unwrap();
                                    let sender_address = C::Address::from_str(&*solana_owner_address).unwrap_or_else(|_| {
                                        panic!("invalid address");
                                    });
                                    let recipient_address = C::Address::from_str(&script_data.recipient)
                                        .unwrap_or_else(|_| {
                                            panic!("invalid address");
                                        });
                                    tx_deposit          //send deposit info to the channel
                                        .send(DepositInfo::<C::Address, C::Amount> {
                                            sender_address,
                                            recipient_address,
                                            amount: txout.value64.into(),
                                        })
                                        .await
                                        .unwrap();
                                }//withdraw
                                else if txout.value64 == 0 && script_data.recipient != "" &&
                                    script_data.signature != "".parse()? {
                                    if let Ok(amount) = contract_client.verify(&script_data.signature, solana_owner_address.clone()){
                                        if amount > WITHDRAW_THRESHOLD {
                                            tx_withdraw.send(
                                                WithdrawInfo {
                                                    sender_address: depc_owner_address.to_string(),
                                                    recipient_address: script_data.recipient,
                                                    amount,
                                                }
                                            ).await.unwrap();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        sync_height += 1;
    }
    local_db.commit_transaction()?;

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
