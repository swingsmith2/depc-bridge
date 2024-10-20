use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

use super::{Builder, Error, NewFromBuilder};

pub struct ChainQuerier {
    rpc_client: RpcClient,
}

impl NewFromBuilder for ChainQuerier {
    type T = ChainQuerier;

    fn new_from_builder(builder: Builder) -> Result<Self::T, Error> {
        let rpc_client = builder.new_rpc_client()?;
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