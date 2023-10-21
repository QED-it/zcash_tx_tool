pub mod reqwest;
mod mock;

use std::error::Error;
use zebra_chain::block::{Block, Hash as BlockHash, Height};
use zebra_chain::transaction::{Transaction, Hash as TxHash};

pub const NODE_URL: &str = "http://127.0.0.1:18232/"; // TODO get from config

pub trait RpcClient {
    fn get_best_block_hash(&self) -> Result<BlockHash, Box<dyn Error>>;
    fn get_block(&self, height: Height) -> Result<Block, Box<dyn Error>>;
    fn send_raw_transaction(&self, tx: Transaction) -> Result<TxHash, Box<dyn Error>>;
}
