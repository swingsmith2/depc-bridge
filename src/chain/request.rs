use anyhow::Result;
use ureq::AgentBuilder;

use super::{Config, RpcJson, RpcResp};

pub fn request(config: &Config, rpc_json: &RpcJson) -> Result<RpcResp> {
    let agent = AgentBuilder::new()
        .try_proxy_from_env(config.use_proxy)
        .build();
    let body = serde_json::to_string_pretty(rpc_json)?;
    let resp = agent
        .post(&config.endpoint)
        .send_string(&body)?
        .into_string()?;
    Ok(serde_json::from_str(&resp)?)
}
