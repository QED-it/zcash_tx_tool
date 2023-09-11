mod reqwest;

use zebra_chain::block::Block;

pub const NODE_URL: &str = "http://zebrad:4242/"; // TODO get from config

pub trait RpcClient {
    fn get_best_block_hash(&self) -> String;
    fn get_block(&self, height: u64) -> Block;
}
