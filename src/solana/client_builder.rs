use std::str::FromStr;

use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};

use super::Client;
use super::Error;

pub struct ClientBuilder {
    url: Option<String>,
    payer: Option<Keypair>,
    contract_address: Option<Pubkey>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self {
            url: None,
            payer: None,
            contract_address: None,
        }
    }

    pub fn build(self) -> Result<Client, Error> {
        if self.url.is_none() {
            return Err(Error::MissingUrl);
        }
        Ok(Client {
            rpc_client: RpcClient::new_with_commitment(
                self.url.unwrap(),
                CommitmentConfig::confirmed(),
            ),
            payer: self.payer,
            contract_address: self.contract_address,
        })
    }

    pub fn set_url<U>(mut self, url: U) -> Self
    where
        U: ToString,
    {
        self.url = Some(url.to_string());
        self
    }

    pub fn set_url_devnet(mut self) -> Self {
        self.url = Some("https://api.devnet.solana.com".to_owned());
        self
    }

    pub fn set_payer_from_base58_string(mut self, s: &str) -> Self {
        self.payer = Some(Keypair::from_base58_string(s));
        self
    }

    pub fn set_contract_address(mut self, s: &str) -> Self {
        self.contract_address = Some(Pubkey::from_str(s).unwrap());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_builder_missing_fields() {
        assert!(ClientBuilder::new().build().is_err());
    }

    #[test]
    fn test_solana_client_builder_complete_fields() {
        let client = ClientBuilder::new()
            .set_url("http://127.0.0.1:8899")
            .build()
            .unwrap();
        let height = client.get_height().unwrap();
        assert!(height > 0);
    }
}