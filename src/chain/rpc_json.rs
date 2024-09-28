use std::vec::Vec;

use serde::Serialize;

#[derive(Serialize)]
pub struct RpcJson {
    jsonrpc: String,
    method: String,
    params: Vec<String>,
    id: u32,
}

pub struct RpcJsonBuilder {
    rpc_json: RpcJson,
}

impl RpcJsonBuilder {
    pub fn new() -> RpcJsonBuilder {
        RpcJsonBuilder {
            rpc_json: RpcJson {
                jsonrpc: "2.0".to_owned(),
                method: "".to_owned(),
                params: vec![],
                id: 0,
            },
        }
    }

    pub fn set_id(mut self, id: u32) -> RpcJsonBuilder {
        self.rpc_json.id = id;
        self
    }

    pub fn set_method(mut self, method_name: &str) -> RpcJsonBuilder {
        self.rpc_json.method = method_name.to_owned();
        self
    }

    pub fn add_param_i64(mut self, value: i64) -> RpcJsonBuilder {
        self.rpc_json.params.push(value.to_string());
        self
    }

    pub fn add_param_string(mut self, value: &str) -> RpcJsonBuilder {
        self.rpc_json.params.push(value.to_owned());
        self
    }

    pub fn build(self) -> RpcJson {
        // TODO we might need to ensure `rpc_json` is valid
        self.rpc_json
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rpc_json_builder_set_method_name() {
        let builder = RpcJsonBuilder::new();
        let rpc_json = builder.set_method("hello").build();
        assert_eq!(rpc_json.method, "hello");
    }

    #[test]
    fn test_rpc_json_builder_add_param_i64() {
        let builder = RpcJsonBuilder::new();
        let rpc_json = builder.add_param_i64(100).build();
        assert_eq!(rpc_json.params.len(), 1);
        assert_eq!(*rpc_json.params.get(0).unwrap(), 100.to_string());
    }

    #[test]
    fn test_rpc_json_builder_add_param_i64_str() {
        let builder = RpcJsonBuilder::new();
        let rpc_json = builder
            .add_param_i64(100)
            .add_param_string("hello world")
            .build();

        assert_eq!(rpc_json.params.len(), 2);
        assert_eq!(*rpc_json.params.get(0).unwrap(), 100.to_string());
        assert_eq!(*rpc_json.params.get(1).unwrap(), "hello world");
    }
}
