use hex::FromHex;
use reqwest::blocking::Client;
use zebra_chain::block::{Block, Hash as BlockHash, Height};
use crate::components::rpc_client::{RpcClient, NODE_URL};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io;
use std::io::ErrorKind;
use serde::de::DeserializeOwned;
use zebra_chain::serialization::{ZcashDeserialize, ZcashSerialize};
use zebra_chain::transaction::{Transaction, Hash as TxHash};
use zebra_rpc::methods::{GetBlock, SentTransactionHash};
use crate::prelude::info;

pub struct ReqwestRpcClient {
    client: Client
}

impl ReqwestRpcClient {
    pub fn new() -> Self {
        Self {
            client: Client::new()
        }
    }

    fn request<T>(&self, request: &RpcRequest) -> Result<T, Box<dyn Error>>
        where T: DeserializeOwned
    {
        let binding = self.client.post(
            NODE_URL
        )
            .body(serde_json::to_string(request)?)
            .send()?
            .text()?;
        let response_string = binding.as_str();

        info!("Request {} Response: {}", request.method, response_string);

        let rpc_result: RpcResponse<T> = serde_json::from_str(response_string)?;

        Ok(rpc_result.result)
    }
}

impl RpcClient for ReqwestRpcClient {

    fn get_best_block_hash(&self) -> Result<BlockHash, Box<dyn Error>> {
        let hash_string: String = self.request(&RpcRequest::new("getbestblockhash"))?;
        let hash = BlockHash::from_hex(hash_string.as_str())?;
        Ok(hash)
    }

    fn get_block(&self, height: Height) -> Result<Block, Box<dyn Error>> {
        let mut params: Vec<ParamType> = Vec::new();
        params.push(ParamType::String(height.0.to_string())); // Height
        params.push(ParamType::Number(0)); // Verbosity
        let block: GetBlock = self.request(&RpcRequest::new_with_params("getblock", params))?;

        match block {
            GetBlock::Raw(bdata) => Ok(Block::zcash_deserialize(bdata.as_ref())?),
            GetBlock::Object { .. } => Err(io::Error::new(ErrorKind::InvalidData, "GetBlock::Object not supported yet").into())
        }
    }

    fn send_raw_transaction(&self, tx: Transaction) -> Result<TxHash, Box<dyn Error>> {
        let mut params: Vec<ParamType> = Vec::new();
        params.push(ParamType::String(hex::encode(tx.zcash_serialize_to_vec().unwrap())));
        let tx_hash: SentTransactionHash = self.request(&RpcRequest::new_with_params("sendrawtransaction", params))?;
        Ok(tx_hash.0)
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum ParamType {
    String(String),
    Number(u32)
}

#[derive(Serialize)]
struct RpcRequest {
    jsonrpc: &'static str,
    id: &'static str,
    method: &'static str,
    params: Vec<ParamType>
}

impl RpcRequest {

    fn new(method: &'static str) -> RpcRequest {
        Self {
            jsonrpc: "1.0",
            id: "zsa-wallet",
            method: method,
            params: Vec::new()
        }
    }

    fn new_with_params(method: &'static str, params: Vec<ParamType>) -> RpcRequest {
        Self {
            jsonrpc: "1.0",
            id: "zsa-wallet",
            method: method,
            params: params
        }
    }
}

#[derive(Deserialize)]
struct RpcResponse<T> {
    id: Box<str>,
    result: T
}