use crate::components::rpc_client::{BlockProposal, BlockTemplate, GetBlock, RpcClient};
use crate::model::Block;
use crate::prelude::info;
use reqwest::blocking::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::error::Error;
use zcash_primitives::block::BlockHash;
use zcash_primitives::consensus::{BlockHeight, BranchId};
use zcash_primitives::transaction::{Transaction, TxId};

pub struct ReqwestRpcClient {
    client: Client,
    node_url: String,
}

impl ReqwestRpcClient {
    pub fn new(node_url: String) -> Self {
        Self {
            client: Client::new(),
            node_url,
        }
    }

    fn request<T>(&self, request: &RpcRequest) -> Result<T, Box<dyn Error>>
    where
        T: DeserializeOwned,
    {
        info!("Request {} Body: {}", request.method, serde_json::to_string(&request.params).unwrap());

        let binding = self
            .client
            .post(&self.node_url)
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
        let mut block_hash_bytes = hex::decode(hash).unwrap();
        block_hash_bytes.reverse();
        Ok(BlockHash(block_hash_bytes.as_slice().try_into().unwrap()))
    }

    fn get_block(&self, height: u32) -> Result<Block, Box<dyn Error>> {
        let params: Vec<ParamType> = vec![
            ParamType::String(height.to_string()), // Height
            ParamType::Number(1),                  // Verbosity
        ];
        let block: GetBlock = self.request(&RpcRequest::new_with_params("getblock", params))?;

        Ok(Block {
            hash: BlockHash(
                hex::decode(block.hash)
                    .unwrap()
                    .as_slice()
                    .try_into()
                    .unwrap(),
            ),
            height: BlockHeight::from_u32(block.height.unwrap()),
            confirmations: block.confirmations,
            tx_ids: block
                .tx
                .iter()
                .map(|tx_id_str| {
                    TxId::from_bytes(
                        hex::decode(tx_id_str)
                            .unwrap()
                            .as_slice()
                            .try_into()
                            .unwrap(),
                    )
                })
                .collect(),
            previous_block_hash: BlockHash([0; 32]), // Previous block hash is not yet implemented in Zebra
        })
    }

    fn send_transaction(&mut self, tx: Transaction) -> Result<TxId, Box<dyn Error>> {
        let mut tx_bytes = vec![];
        tx.write(&mut tx_bytes).unwrap();

        let tx_hash: String = self.request(&RpcRequest::new_with_params(
            "sendrawtransaction",
            vec![ParamType::String(hex::encode(tx_bytes))],
        ))?;
        let tx_hash_bytes: [u8; 32] = hex::decode(tx_hash).unwrap().as_slice().try_into().unwrap();
        Ok(TxId::from_bytes(tx_hash_bytes))
    }

    fn get_transaction(&self, txid: &TxId) -> Result<Transaction, Box<dyn Error>> {
        let params: Vec<ParamType> = vec![
            ParamType::String(hex::encode(txid.as_ref())), // TxId
            ParamType::Number(0),                          // Verbosity
        ];
        let tx_hex: String =
            self.request(&RpcRequest::new_with_params("getrawtransaction", params))?;
        let tx_bytes = hex::decode(tx_hex).unwrap();
        Ok(Transaction::read(tx_bytes.as_slice(), BranchId::Nu5).unwrap())
    }

    fn get_block_template(&self) -> Result<BlockTemplate, Box<dyn Error>> {
        self.request(&RpcRequest::new("getblocktemplate"))
    }

    fn submit_block(&self, block: BlockProposal) -> Result<Option<String>, Box<dyn Error>> {
        let mut block_bytes = vec![];
        block.write(&mut block_bytes).unwrap();

        let result = self.request(&RpcRequest::new_with_params(
            "submitblock",
            vec![ParamType::String(hex::encode(block_bytes))],
        ))?;

        match result {
            None => Ok(None),

            Some(result) => {
                if result == "rejected" {
                    Err("Block rejected".into())
                } else {
                    Ok(Some(result))
                }
            }
        }
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum ParamType {
    String(String),
    Number(u32),
}

#[derive(Serialize)]
struct RpcRequest {
    jsonrpc: &'static str,
    id: &'static str,
    method: &'static str,
    params: Vec<ParamType>,
}

impl RpcRequest {
    fn new(method: &'static str) -> RpcRequest {
        Self {
            jsonrpc: "1.0",
            id: "zcash-tx-tool",
            method,
            params: Vec::new(),
        }
    }

    fn new_with_params(method: &'static str, params: Vec<ParamType>) -> RpcRequest {
        Self {
            jsonrpc: "1.0",
            id: "zcash-tx-tool",
            method,
            params,
        }
    }
}

#[derive(Deserialize)]
struct RpcResponse<T> {
    result: T,
}
