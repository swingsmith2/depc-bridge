use ethers::prelude::*;

pub use ethers::types::Address;

pub struct Config {
    pub provider: Provider<Http>,
    pub contract_address: Address,
    pub wallet: LocalWallet,
}

pub struct Bridge {
    config: Config,
}

impl Bridge {
    fn new(config: Config) -> Bridge {
        Bridge { config }
    }
}

#[derive(Debug)]
pub enum BuilderError {
    InvalidEndpoint,
    InvalidAddress,
    InvalidPrivateKey,
    CannotRetrieveChainId,
    MissingFieldInConfig,
}

pub struct BridgeBuilder {
    provider: Option<Provider<Http>>,
    contract_address: Option<Address>,
    wallet: Option<LocalWallet>,
}

impl BridgeBuilder {
    pub fn new() -> BridgeBuilder {
        BridgeBuilder {
            provider: None,
            contract_address: None,
            wallet: None,
        }
    }

    pub fn build(self) -> Result<Bridge, BuilderError> {
        if self.provider.is_none() || self.contract_address.is_none() || self.wallet.is_none() {
            return Err(BuilderError::MissingFieldInConfig);
        }
        Ok(Bridge::new(Config {
            provider: self.provider.unwrap(),
            contract_address: self.contract_address.unwrap(),
            wallet: self.wallet.unwrap(),
        }))
    }

    pub fn set_endpoint(mut self, endpoint_str: &str) -> Result<BridgeBuilder, BuilderError> {
        if let Ok(provider) = Provider::<Http>::try_from(endpoint_str) {
            self.provider = Some(provider);
            Ok(self)
        } else {
            Err(BuilderError::InvalidEndpoint)
        }
    }

    pub fn set_contract_address(
        mut self,
        address_str: &str,
    ) -> Result<BridgeBuilder, BuilderError> {
        if let Ok(address) = address_str.parse::<Address>() {
            self.contract_address = Some(address);
            Ok(self)
        } else {
            Err(BuilderError::InvalidAddress)
        }
    }

    pub fn set_wallet_private_key(
        mut self,
        private_key_str: &str,
        chain_id: u64,
    ) -> Result<BridgeBuilder, BuilderError> {
        if let Ok(wallet) = private_key_str.parse::<LocalWallet>() {
            self.wallet = Some(wallet.with_chain_id(chain_id));
            Ok(self)
        } else {
            Err(BuilderError::InvalidPrivateKey)
        }
    }
}

pub async fn retrieve_chain_id(endpoint_str: &str) -> Result<U256, BuilderError> {
    if let Ok(provider) = Provider::<Http>::try_from(endpoint_str) {
        if let Ok(chain_id) = provider.get_chainid().await {
            Ok(chain_id)
        } else {
            Err(BuilderError::CannotRetrieveChainId)
        }
    } else {
        Err(BuilderError::InvalidEndpoint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bridge_retrieve_chain_id() {
        let chain_id =
            retrieve_chain_id("https://sepolia.infura.io/v3/daad1c45f9b6487288f56ff2bac9577a")
                .await
                .unwrap();
        assert_eq!(chain_id.as_u64(), 11155111);
    }
}