pub mod reqwest;
pub mod mock;

use std::error::Error;
use zcash_primitives::block::BlockHash;
use zcash_primitives::transaction::{Transaction, TxId};
use crate::model::Block;


pub const NODE_URL: &str = "http://127.0.0.1:18232/"; // TODO get from config

pub trait RpcClient {
    fn get_best_block_hash(&self) -> Result<BlockHash, Box<dyn Error>>;
    fn get_block(&self, height: u32) -> Result<Block, Box<dyn Error>>;
    fn send_transaction(&mut self, tx: Transaction) -> Result<TxId, Box<dyn Error>>;
    fn get_transaction(&self, txid: TxId) -> Result<Transaction, Box<dyn Error>>;
}
