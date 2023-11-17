pub mod reqwest;
pub mod mock;

use std::error::Error;
use zcash_primitives::block::BlockHash;
use zcash_primitives::transaction::{Transaction, TxId};
use crate::model::Block;


pub const NODE_URL: &str = "http://rpcuser💻0:rpcpass🔑0@127.0.0.1:18873/"; // TODO get from config

pub trait RpcClient {
    fn get_best_block_hash(&self) -> Result<BlockHash, Box<dyn Error>>;
    fn get_block(&self, height: u32) -> Result<Block, Box<dyn Error>>;
    fn send_transaction(&mut self, tx: Transaction) -> Result<TxId, Box<dyn Error>>;
    fn get_transaction(&self, txid: TxId, block_id: &BlockHash) -> Result<Transaction, Box<dyn Error>>;
}

/// =========================== Messages ===========================

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GetBlock {
        /// The hash of the requested block in hex.
        hash: String,

        /// The number of confirmations of this block in the best chain,
        /// or -1 if it is not in the best chain.
        confirmations: i64,

        /// The height of the requested block.
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<u32>,

        /// List of transaction IDs in block order, hex-encoded.
        //
        // TODO: use a typed Vec<transaction::Hash> here
        tx: Vec<String>
}

