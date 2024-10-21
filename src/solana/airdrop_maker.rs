use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

use super::{Builder, ChainQuerier, Error, NewFromBuilder};

pub struct AirdropMaker {
    rpc_client: RpcClient,
    target_pubkey: Pubkey,
}

impl NewFromBuilder for AirdropMaker {
    type T = AirdropMaker;

    fn new_from_builder(builder: Builder) -> Result<Self::T, super::Error> {
        let rpc_client = builder.create_rpc_client_from_url()?;
        if builder.target_pubkey.is_none() {
            return Err(Error::MissingRequiredField);
        }
        let target_pubkey = builder.target_pubkey.unwrap();
        Ok(AirdropMaker {
            rpc_client,
            target_pubkey,
        })
    }
}

impl AirdropMaker {
    pub fn airdrop(&self, amount: u64) -> Result<Signature, Error> {
        let res = self.rpc_client.request_airdrop(&self.target_pubkey, amount);
        if let Err(e) = res {
            println!("cannot request airdrop, reason: {}", e);
            return Err(Error::MissingRequiredField);
        }
        let signature = res.unwrap();
        Ok(signature)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_airdrop() {
        const AIRDROP_AMOUNT: u64 = 1_000_000_000;
        let airdrop_pubkey =
            Pubkey::from_str("CF2XGuxaYcmg5Li8pYUdd9C1UtGe9amSG3TVM2A1PuXR").unwrap();
        let chain_querier = Builder::new()
            .set_url_localhost()
            .build::<ChainQuerier>()
            .unwrap();
        let balance_before_airdrop = chain_querier.get_balance(&airdrop_pubkey).unwrap();
        println!(
            "aidrop to public-key: {}, current balance: {}",
            airdrop_pubkey, balance_before_airdrop
        );
        let airdrop_maker = Builder::new()
            .set_url_localhost()
            .set_target_pubkey(airdrop_pubkey)
            .build::<AirdropMaker>()
            .unwrap();
        let signature = airdrop_maker.airdrop(AIRDROP_AMOUNT).unwrap();
        println!("airdrop signature: {}", signature);
        // wait until the tx is on-chain
        chain_querier.wait_tx(signature).unwrap();
        // check balance
        let balance = chain_querier.get_balance(&airdrop_pubkey).unwrap();
        assert_eq!(balance, balance_before_airdrop + AIRDROP_AMOUNT);
    }
}