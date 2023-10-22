use std::convert::TryInto;
use reqwest::blocking::Client;
use crate::components::rpc_client::{RpcClient, NODE_URL};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io;
use std::io::ErrorKind;
use serde::de::DeserializeOwned;
use zcash_primitives::block::BlockHash;
use zcash_primitives::consensus::{BlockHeight, BranchId};
use zcash_primitives::transaction::{Transaction, TxId};
use zebra_rpc::methods::{GetBlock, GetRawTransaction, SentTransactionHash};
use crate::model::Block;
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
        let hash: String = self.request(&RpcRequest::new("getbestblockhash"))?;
        Ok(BlockHash::from_slice(hex::decode(hash).unwrap().as_slice()))
    }

    fn get_block(&self, height: u32) -> Result<Block, Box<dyn Error>> {
        let mut params: Vec<ParamType> = Vec::new();
        params.push(ParamType::String(height.to_string())); // Height
        params.push(ParamType::Number(2)); // Verbosity
        let block: GetBlock = self.request(&RpcRequest::new_with_params("getblock", params))?;

        match block {
            GetBlock::Raw(_) => Err(io::Error::new(ErrorKind::InvalidData, "GetBlock::Raw is not supported").into()),
            GetBlock::Object{ hash, confirmations, height, tx, trees } => Ok(Block {
                hash: BlockHash(hash.0.0),
                height: BlockHeight::from_u32(height.unwrap().0),
                confirmations: confirmations,
                tx_ids: tx.iter().map(|tx_id_str| TxId::from_bytes(hex::decode(tx_id_str).unwrap().as_slice().try_into().unwrap())).collect(),
                previous_block_hash: BlockHash([0; 32]) // TODO add previous block hash to Getblock RPC
            })
        }
    }

    fn send_transaction(&self, tx: Transaction) -> Result<TxId, Box<dyn Error>> {
        let mut tx_bytes = vec![];
        tx.write(&mut tx_bytes).unwrap();

        let mut params: Vec<ParamType> = Vec::new();
        params.push(ParamType::String(hex::encode(tx_bytes)));
        let tx_hash: SentTransactionHash = self.request(&RpcRequest::new_with_params("sendrawtransaction", params))?;
        Ok(TxId::from_bytes(tx_hash.0.0))
    }

    fn get_transaction(&self, txid: TxId) -> Result<Transaction, Box<dyn Error>> {
        let mut params: Vec<ParamType> = Vec::new();
        params.push(ParamType::String(hex::encode(txid.as_ref())));
        let rpc_tx: GetRawTransaction = self.request(&RpcRequest::new_with_params("getrawtransaction", params))?;

        match rpc_tx {
            GetRawTransaction::Raw(txdata) =>  Ok(Transaction::read(txdata.as_ref(), BranchId::Nu5).unwrap()),
            GetRawTransaction::Object { .. } => Err(io::Error::new(ErrorKind::InvalidData, "GetBlock::Raw is not supported").into()),
        }
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