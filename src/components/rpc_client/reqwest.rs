use std::any::Any;
use std::collections::HashMap;
use reqwest::blocking::Client;
use zebra_chain::block::Block;
use crate::components::rpc_client::{RpcClient, NODE_URL};

pub struct ReqwestRpcClient {
    client: Client
}

impl ReqwestRpcClient {
    fn new() -> Self {
        Self {
            client: Client::new()
        }
    }
}

impl RpcClient for ReqwestRpcClient {

    fn get_best_block_hash(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(
            self.client.post(
                NODE_URL
            )
                .body(RpcRequest::new("getbestblockhash"))
                .send()?
                .json::<String>()?
        )
    }

    fn get_block(&self, height: u64) -> Result<Block, Box<dyn std::error::Error>> {
        let params: HashMap<str, dyn Any> = HashMap::new();
        params["height"] = height;

        Ok(
            self.client.post(
                NODE_URL
            )
                .body(RpcRequest::new_with_params("get_block", params))
                .send()?
                .json::<Block>()?
        )
    }
}

struct RpcRequest {
    jsonrpc: &'static str,
    id: &'static str,
    method: &'static str,
    params: HashMap<str, dyn Any>
}

impl RpcRequest {

    fn new(method: &'static str) -> RpcRequest {
        Self {
            jsonrpc: "1.0",
            id: "zsa-wallet",
            method: method,
            params: HashMap::new()
        }
    }

    fn new_with_params(method: &'static str, params: HashMap<str, dyn Any>) -> RpcRequest {
        Self {
            jsonrpc: "1.0",
            id: "zsa-wallet",
            method: method,
            params: params
        }
    }
}