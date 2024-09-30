use std::collections::HashMap;
use std::rc::Rc;
use std::vec::Vec;

use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct RpcJson {
    jsonrpc: String,
    method: String,
    params: HashMap<String, Value>,
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
                params: HashMap::new(),
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

    pub fn add_param_i64(mut self, name: &str, value: i64) -> RpcJsonBuilder {
        self.rpc_json
            .params
            .insert(name.to_owned(), Value::Number(value.into()));
        self
    }

    pub fn add_param_string(mut self, name: &str, value: &str) -> RpcJsonBuilder {
        self.rpc_json
            .params
            .insert(name.to_owned(), Value::String(value.to_owned()));
        self
    }

    pub fn add_param_bool(mut self, name: &str, value: bool) -> RpcJsonBuilder {
        self.rpc_json
            .params
            .insert(name.to_owned(), Value::Bool(value));
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
        let rpc_json = builder.add_param_i64("number", 100).build();
        assert_eq!(rpc_json.params.len(), 1);
        assert_eq!(*rpc_json.params.get("number").unwrap(), 100);
    }

    #[test]
    fn test_rpc_json_builder_add_param_i64_str() {
        let builder = RpcJsonBuilder::new();
        let rpc_json = builder
            .add_param_i64("number", 100)
            .add_param_string("string", "hello world")
            .build();

        assert_eq!(rpc_json.params.len(), 2);
        assert_eq!(*rpc_json.params.get("number").unwrap(), 100);
        assert_eq!(*rpc_json.params.get("string").unwrap(), "hello world");
    }
}
