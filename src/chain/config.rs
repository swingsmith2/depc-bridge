pub struct Config {
    pub endpoint: String,
    pub use_proxy: bool,
}

impl Config {
    pub fn new() -> Config {
        Config {
            endpoint: "127.0.0.1:18732".to_owned(),
            use_proxy: true,
        }
    }

    pub fn set_endpoint(mut self, endpoint: String) -> Config {
        self.endpoint = endpoint;
        self
    }

    pub fn set_use_proxy(mut self, u: bool) -> Config {
        self.use_proxy = u;
        self
    }
}
