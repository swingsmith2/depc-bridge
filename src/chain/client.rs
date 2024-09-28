use anyhow::Result;

use super::{request, Config, RpcJson, RpcJsonBuilder};

pub struct Client {
    config: Config,
}

impl Client {
    pub fn get_height(&self) -> Result<u64> {
        let rpc_json = RpcJsonBuilder::new().set_method("getblockcount").build();
        let resp = request(&self.config, &rpc_json)?;
        Ok(resp.result.as_u64().unwrap_or_default())
    }
}

pub struct ClientBuilder {
    endpoint: String,
    use_proxy: bool,
    rpc_json: RpcJson,
}

impl ClientBuilder {
    pub fn new() -> ClientBuilder {
        ClientBuilder {
            endpoint: "127.0.0.1:18732".to_owned(),
            use_proxy: false,
            rpc_json: RpcJsonBuilder::new().build(),
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

    pub fn set_rpc_json(mut self, rpc_json: RpcJson) -> ClientBuilder {
        self.rpc_json = rpc_json;
        self
    }

    pub fn build(self) -> Client {
        Client {
            config: Config {
                endpoint: self.endpoint,
                use_proxy: self.use_proxy,
            },
        }
    }
}
