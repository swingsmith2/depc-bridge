use std::fs;

use anyhow::Result;

use super::{request, Auth, Config, RpcJsonBuilder};

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

    pub fn set_auth(mut self, user: &str, passwd: &str) -> ClientBuilder {
        let auth_str = format!("{user}:{passwd}");
        self.auth = Some(format!("Basic {}", rbase64::encode(auth_str.as_bytes())));
        self
    }

    pub fn set_auth_from_cookie(mut self, cookie_path: &str) -> ClientBuilder {
        let auth_str = fs::read_to_string(cookie_path).unwrap();
        let auth: Auth = auth_str.try_into().unwrap();
        self.set_auth(&auth.user, &auth.passwd)
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
        // the cookie file for testnet
        let cookie_path = shellexpand::env("$HOME/.depinc/testnet3/.cookie").unwrap();

        let builder = ClientBuilder::new();
        let client = builder.set_auth_from_cookie(&cookie_path).build();
        let height = client.get_height().unwrap();
        assert_ne!(height, 0);
    }
}
