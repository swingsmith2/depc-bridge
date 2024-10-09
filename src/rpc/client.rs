use log::debug;

use anyhow::Result;
use ureq::AgentBuilder;

use super::{Config, Request, Response};

pub struct Client {
    config: Config,
}

impl Client {
    pub fn new(config: Config) -> Client {
        Client { config }
    }

    pub fn send(&self, req: &Request) -> Result<Response> {
        let agent = AgentBuilder::new()
            .try_proxy_from_env(self.config.use_proxy)
            .build();
        let body = serde_json::to_string_pretty(req)?;
        let mut req = agent.post(&self.config.endpoint);
        if let Some(auth) = &self.config.auth {
            req = req.set("Authorization", auth);
        }
        debug!("sending body:\n{}\n", body);
        let resp = req.send_string(&body)?;
        let resp_str = resp.into_string()?;
        Ok(serde_json::from_str(&resp_str)?)
    }
}
