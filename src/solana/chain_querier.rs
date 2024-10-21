use std::thread::sleep;
use std::time::Duration;

use solana_client::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature};

use super::{Builder, Error, NewFromBuilder};

pub struct ChainQuerier {
    rpc_client: RpcClient,
}

impl NewFromBuilder for ChainQuerier {
    type T = ChainQuerier;

    fn new_from_builder(builder: Builder) -> Result<Self::T, Error> {
        let rpc_client = builder.create_rpc_client_from_url()?;
        Ok(ChainQuerier { rpc_client })
    }
}

impl ChainQuerier {
    pub fn get_height(&self) -> Result<u64, Error> {
        if let Ok(height) = self.rpc_client.get_block_height() {
            Ok(height)
        } else {
            Err(Error::CannotGetBlockHeight)
        }
    }

    pub fn get_balance(&self, pubkey: &Pubkey) -> Result<u64, Error> {
        let res = self.rpc_client.get_balance(pubkey);
        if let Err(e) = res {
            println!("cannot retrieve balance, reason: {}", e);
            return Err(Error::CannotGetAccountBalance);
        }
        let balance = res.unwrap();
        Ok(balance)
    }

    pub fn wait_tx(&self, signature: Signature) -> Result<(), Error> {
        loop {
            let res = match self
                .rpc_client
                .get_signature_status_with_commitment(&signature, CommitmentConfig::confirmed())
            {
                Ok(s) => {
                    if s.is_some() {
                        // ok, the tx is processed
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
                Err(e) => {
                    println!("cannot get status for signature, reason: {}", e);
                    return Err(Error::CannotGetStatusForSignature);
                }
            };
            if res.is_ok() {
                let succ = res.unwrap();
                if succ {
                    break;
                } else {
                    sleep(Duration::from_secs(1));
                }
            } else {
                return res.expect_err("this should be an error");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_height() {
        let querier = Builder::new()
            .set_url_localhost()
            .build::<ChainQuerier>()
            .unwrap();
        let height = querier.get_height().unwrap();
        assert!(height > 0);
    }
}