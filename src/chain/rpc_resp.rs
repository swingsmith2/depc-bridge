use serde::Deserialize;
use serde_json::{Error, Value};

#[derive(Deserialize)]
pub struct RpcResp {
    pub jsonrpc: String,
    pub id: u32,
    pub result: Value,
}

pub fn parse_str(s: &str) -> Result<RpcResp, Error> {
    serde_json::from_str(s)
}

#[cfg(test)]
mod test {
    use super::*;

    const STANDARD_JSON_RPC: &str = r#"
        {"jsonrpc": "2.0", "result": "hello world", "id": 0}
    "#;

    #[test]
    fn test_rpc_resp_parse_json_rpc() {
        assert!(parse_str(STANDARD_JSON_RPC).is_ok());
        assert_eq!(parse_str(STANDARD_JSON_RPC).unwrap().id, 0);
        assert_eq!(parse_str(STANDARD_JSON_RPC).unwrap().jsonrpc, "2.0");
        assert_eq!(parse_str(STANDARD_JSON_RPC).unwrap().result, "hello world");
    }
}
