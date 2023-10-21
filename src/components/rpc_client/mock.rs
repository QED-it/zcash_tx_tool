// use std::io;
// use std::io::ErrorKind;
// use std::error::Error;
// use zebra_chain::block::{Block, Hash, Height};
// use crate::components::rpc_client::{RpcClient};
//
//
// pub struct MockZcashNode {
//     blockchain: Vec<Block>,
// }
//
// impl MockZcashNode {
//     pub fn new() -> Self {
//         Self {
//             blockchain: Vec::new(),
//         }
//     }
// }
//
// impl RpcClient for MockZcashNode {
//
//     fn get_best_block_hash(&self) -> Result<Hash, Box<dyn Error>> {
//         self.blockchain.last().ok_or(io::Error::new(ErrorKind::NotFound, "Block not found").into()).map(|b| b.hash())
//     }
//
//     fn get_block(&self, height: Height) -> Result<Block, Box<dyn Error>> {
//         Ok(self.blockchain[height.0 as usize].clone())
//     }
// }