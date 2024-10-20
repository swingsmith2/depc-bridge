use solana_client::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair};

use super::Error;

pub trait NewFromBuilder {
    type T;
    fn new_from_builder(builder: Builder) -> Result<Self::T, Error>;
}

pub struct Builder {
    pub(crate) url: Option<String>,
    pub(crate) authority_key: Option<Keypair>,
    pub(crate) mint_key: Option<Keypair>,
    pub(crate) mint_pubkey: Option<Pubkey>,
    pub(crate) target_pubkey: Option<Pubkey>,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            url: None,
            authority_key: None,
            mint_key: None,
            mint_pubkey: None,
            target_pubkey: None,
        }
    }

    pub fn build<T>(self) -> Result<T::T, Error>
    where
        T: NewFromBuilder,
    {
        T::new_from_builder(self)
    }

    pub fn new_rpc_client(&self) -> Result<RpcClient, Error> {
        if self.url.is_none() {
            return Err(Error::MissingRequiredField);
        }
        Ok(RpcClient::new_with_commitment(
            self.url.clone().unwrap(),
            CommitmentConfig::confirmed(),
        ))
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

    pub fn set_url_localhost(mut self) -> Self {
        self.url = Some("http://127.0.0.1:8899".to_owned());
        self
    }

    pub fn set_authority_key(mut self, authority_key: Keypair) -> Self {
        self.authority_key = Some(authority_key);
        self
    }

    pub fn set_random_authority_key(self) -> Self {
        self.set_authority_key(Keypair::new())
    }

    pub fn set_mint_key(mut self, mint_key: Keypair) -> Self {
        self.mint_key = Some(mint_key);
        self
    }

    pub fn set_random_mint_key(self) -> Self {
        self.set_mint_key(Keypair::new())
    }

    pub fn set_mint_pubkey(mut self, mint_pubkey: Pubkey) -> Self {
        self.mint_pubkey = Some(mint_pubkey);
        self
    }

    pub fn set_target_pubkey(mut self, target_pubkey: Pubkey) -> Self {
        self.target_pubkey = Some(target_pubkey);
        self
    }
}