use alloy::{
    providers::{Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
};

pub enum BuilderError {
    InvalidEndpoint,
    InvalidPrivateKey,
    MissingField,
}

pub struct DeployerBuilder {
    provider: Option<Provider>,
    signer: Option<PrivateKeySigner>,
}

impl DeployerBuilder {
    pub fn new() -> DeployerBuilder {
        DeployerBuilder {
            provider: None,
            signer: None,
        }
    }

    pub fn build(self) -> Result<Deployer, BuilderError> {
        if self.provider.is_none() || self.signer.is_none() {
            return Err(BuilderError::MissingField);
        }
        Ok(Deployer {
            provider: self.provider.unwrap(),
            signer: self.signer.unwrap(),
        })
    }

    pub async fn set_endpoint(mut self, endpoint: &str) -> Result<Self, BuilderError> {
        if let Ok(provider) = ProviderBuilder::new()
            .with_recommended_fillers()
            .on_builtin(endpoint)
            .await
        {
            self.provider = Some(provider);
            Ok(self)
        } else {
            Err(BuilderError::InvalidEndpoint)
        }
    }

    pub fn set_private_key(
        mut self,
        private_key_str: &str,
        chain_id: u64,
    ) -> Result<Self, BuilderError> {
        if let Ok(private_key) = private_key_str.parse::<PrivateKeySigner>() {
            self.signer = Some(private_key);
            Ok(self)
        } else {
            Err(BuilderError::InvalidPrivateKey)
        }
    }
}

pub enum Error {}

pub struct Deployer {
    provider: Provider<Http>,
    signer: PrivateKeySigner,
}

impl Deployer {
    pub fn deploy(&self, contract_abi: &str) -> Result<H256, Error> {}
}
