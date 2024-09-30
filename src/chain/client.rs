use std::fs;

use log::error;

use anyhow::Result;

use super::{req, Block, Config, Error, RpcJsonBuilder};

pub struct Client {
    config: Config,
}

impl Client {
    pub fn get_height(&self) -> Result<u64> {
        let rpc_json = RpcJsonBuilder::new().set_method("getblockcount").build();
        let resp = req(&self.config, &rpc_json)?;
        Ok(resp.result.as_u64().unwrap_or_default())
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

    pub fn set_auth_from_cookie(mut self, cookie_path: &str) -> ClientBuilder {
        let auth_str = fs::read_to_string(cookie_path).unwrap();
        self.set_auth(&auth_str)
    }

    pub fn set_auth_from_default_cookie(mut self, testnet3: bool) -> ClientBuilder {
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
        env_logger::init();
        let builder = ClientBuilder::new();
        let client = builder.set_auth_from_default_cookie(true).build();
        let block_hash = client.get_block_hash(0).unwrap();
        assert_eq!(
            block_hash,
            "8cec494f7f02ad25b3abf418f7d5647885000e010c34e16c039711e4061497b0"
        );
    }
}
