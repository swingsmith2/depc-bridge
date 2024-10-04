use std::sync::{Arc, Mutex};

use anyhow::Result;
use log::info;

use crate::chain::Client;
use crate::db::Conn;

pub async fn sync(
    conn: &Conn,
    client: &Client,
    owner_address: &str,
    exit_sig: Arc<Mutex<bool>>,
) -> Result<()> {
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
        let chain_height = client.get_height()?;
        if sync_height > chain_height {
            // there is no more block left to sync
            break;
        }
        info!(
            "syncing from height {sync_height} to chain height {chain_height}, distance {}",
            chain_height - sync_height
        );

        // block
        let block_hash = client.get_block_hash(sync_height)?;
        let block = client.get_block(&block_hash)?;
        assert_eq!(block.height, sync_height);

        conn.add_block(&block.hash, sync_height, &block.miner, block.time)?;

        if sync_height > 0 {
            // transactions
            for txid in block.tx.iter() {
                let transaction = client.get_transaction(txid)?;
                assert_eq!(transaction.txid, *txid);
                conn.add_transaction(&block_hash, txid)?;
                for txin in transaction.vin.iter() {
                    if !txin.is_coinbase() {
                        // TODO maybe we need to check the validation of the txin?
                        conn.mark_coin_to_spent(
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
                        conn.add_coin(
                            txid,
                            txout.n,
                            txout.value64,
                            &address,
                            &txout.script_pubkey.hex,
                        )?;
                        // check the coin is mine?
                        if address == owner_address {
                            // TODO now check the extra fields to get the ERC20 info.
                        }
                    }
                }
            }
        }

        sync_height += 1;
    }
    conn.commit_transaction()?;

    Ok(())
}
