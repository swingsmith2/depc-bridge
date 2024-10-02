use std::fs;

use log::error;

use super::{req, Block, Config, Error, RpcJsonBuilder, Transaction};

pub struct Client {
    config: Config,
}

impl Client {
    pub fn get_height(&self) -> Result<u32, Error> {
        let rpc_json = RpcJsonBuilder::new().set_method("getblockcount").build();
        match req(&self.config, &rpc_json) {
            Ok(resp) => Ok(resp.result.as_u64().unwrap() as u32),
            Err(e) => {
                error!("cannot execute `getheight`, reason: {e}");
                Err(Error::General)
            }
        }
    }

    pub fn get_block_hash(&self, height: u32) -> Result<String, Error> {
        let rpc_json = RpcJsonBuilder::new()
            .set_method("getblockhash")
            .add_param_i64("height", height as i64)
            .build();
        match req(&self.config, &rpc_json) {
            Ok(resp) => Ok(resp.result.as_str().unwrap().to_owned()),
            Err(e) => {
                error!("cannot execute `getblockhash`, reason: {e}");
                // Err(Error::General)
                Ok("".to_owned())
            }
        }
    }

    pub fn get_block(&self, block_hash: &str) -> Result<Block, Error> {
        let rpc_json = RpcJsonBuilder::new()
            .set_method("getblock")
            .add_param_string("blockhash", block_hash)
            .build();
        match req(&self.config, &rpc_json) {
            Ok(resp) => Ok(serde_json::from_value(resp.result).unwrap()),
            Err(e) => {
                error!("cannot execute `getblock`, reason: {e}");
                Err(Error::General)
            }
        }
    }

    pub fn get_transaction(&self, txid: &str) -> Result<Transaction, Error> {
        let rpc_json = RpcJsonBuilder::new()
            .set_method("getrawtransaction")
            .add_param_string("txid", txid)
            .add_param_bool("verbose", true)
            .build();
        match req(&self.config, &rpc_json) {
            Ok(resp) => Ok(serde_json::from_value(resp.result).unwrap()),
            Err(e) => {
                error!("cannot execute `getblock`, reason: {e}");
                Err(Error::General)
            }
        }
    }
}

pub struct ClientBuilder {
    endpoint: String,
    use_proxy: bool,
    auth: Option<String>,
}

impl ClientBuilder {
    pub fn new() -> ClientBuilder {
        ClientBuilder {
            endpoint: "http://127.0.0.1:18732".to_owned(),
            use_proxy: false,
            auth: None,
        }
    }

    pub fn set_endpoint(mut self, endpoint: &str) -> ClientBuilder {
        self.endpoint = endpoint.to_owned();
        self
    }

    pub fn set_use_proxy(mut self, use_proxy: bool) -> ClientBuilder {
        self.use_proxy = use_proxy;
        self
    }

    pub fn set_auth(mut self, auth_str: &str) -> ClientBuilder {
        self.auth = Some(format!("Basic {}", rbase64::encode(auth_str.as_bytes())));
        self
    }

    pub fn set_auth_from_cookie(self, cookie_path: &str) -> ClientBuilder {
        let auth_str = fs::read_to_string(cookie_path).unwrap();
        self.set_auth(&auth_str)
    }

    pub fn set_auth_from_default_cookie(self, testnet3: bool) -> ClientBuilder {
        let cookie_path = if testnet3 {
            shellexpand::env("$HOME/.depinc/testnet3/.cookie").unwrap()
        } else {
            shellexpand::env("$HOME/.depinc/.cookie").unwrap()
        };
        self.set_auth_from_cookie(&cookie_path)
    }

    pub fn build(self) -> Client {
        Client {
            config: Config {
                endpoint: self.endpoint,
                use_proxy: self.use_proxy,
                auth: self.auth,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_height() {
        let builder = ClientBuilder::new();
        let client = builder.set_auth_from_default_cookie(true).build();
        let height = client.get_height().unwrap();
        assert_ne!(height, 0);
    }

    #[test]
    fn test_get_block_hash_height_0() {
        let builder = ClientBuilder::new();
        let client = builder.set_auth_from_default_cookie(true).build();
        let block_hash = client.get_block_hash(0).unwrap();
        assert_eq!(
            block_hash,
            "8cec494f7f02ad25b3abf418f7d5647885000e010c34e16c039711e4061497b0"
        );
    }

    #[test]
    fn test_get_block_10000() {
        let builder = ClientBuilder::new();
        let client = builder.set_auth_from_default_cookie(true).build();
        let block_hash = client.get_block_hash(10000).unwrap();
        let block = client.get_block(&block_hash).unwrap();
        assert_eq!(
            block.hash,
            "23bb612184a355f9492f526092b3e3aab6266365e117282de6f1f3a999a96c00"
        );
        assert_eq!(block.height, 10000);
        assert_eq!(block.miner, "2NGWAccrksGM4TmefLN4qyW1kV7VpMngtBQ");
        assert_eq!(block.time, 1531302789);
        assert_eq!(block.tx.len(), 1);
    }

    #[test]
    fn test_get_transaction_838b6158772219d547df240b005c3572c9f15fba0f29be3a92b0e4326c2b33e0() {
        let builder = ClientBuilder::new();
        let client = builder.set_auth_from_default_cookie(true).build();
        let transaction = client
            .get_transaction("838b6158772219d547df240b005c3572c9f15fba0f29be3a92b0e4326c2b33e0")
            .unwrap();
        assert_eq!(
            transaction.txid,
            "838b6158772219d547df240b005c3572c9f15fba0f29be3a92b0e4326c2b33e0"
        );
        assert_eq!(transaction.vin.len(), 1);
        assert_eq!(transaction.vout.len(), 3);
        assert_eq!(transaction.vout.get(1).unwrap().value, 25.0f64);
    }

    #[test]
    fn test_get_transaction_751cbbfefdd1e78950f1e69c79ec96babc3bb44737c587fdd49f86afa6c6234b() {
        let builder = ClientBuilder::new();
        let client = builder.set_auth_from_default_cookie(true).build();
        let transaction = client
            .get_transaction("751cbbfefdd1e78950f1e69c79ec96babc3bb44737c587fdd49f86afa6c6234b")
            .unwrap();
        assert_eq!(
            transaction.txid,
            "751cbbfefdd1e78950f1e69c79ec96babc3bb44737c587fdd49f86afa6c6234b"
        );
        if let Some(in_rec) = transaction.vin.get(0) {
            if let Some(txid) = &in_rec.txid {
                assert_eq!(
                    txid,
                    "480211584a1b7588e94efb02fe7e4e3e5fd1a4ed931d046e2801ba596a634c06"
                );
            }
            return;
        }
        assert!(false);
    }
}