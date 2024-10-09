use serde::Deserialize;
use serde_json::Value;

#[cfg(test)]
use serde_json::Error;

#[derive(Deserialize)]
pub struct Response {
    #[cfg(test)]
    pub jsonrpc: Option<String>,
    #[cfg(test)]
    pub id: u32,
    pub result: Value,
}

#[cfg(test)]
pub fn parse_str(s: &str) -> Result<Response, Error> {
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
        assert_eq!(
            parse_str(STANDARD_JSON_RPC).unwrap().jsonrpc,
            Some("2.0".to_owned())
        );
        assert_eq!(parse_str(STANDARD_JSON_RPC).unwrap().result, "hello world");
    }
}
