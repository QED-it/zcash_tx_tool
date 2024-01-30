use abscissa_core::{Command, Runnable};
use zcash_primitives::transaction::{Transaction, TxId};
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::rpc_client::{RpcClient, template_into_proposal};
use crate::prelude::*;
use crate::components::wallet::Wallet;


/// `mine` subcommand
#[derive(clap::Parser, Command, Debug)]
pub struct MineCmd {
    num_blocks: Option<u32>
}

impl Runnable for MineCmd {
    fn run(&self) {
        let config = APP.config();

        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        let mut wallet = Wallet::new(&config.wallet.seed_phrase);

        let num_blocks = self.num_blocks.unwrap_or(1);

        let (_, coinbase_txid) = mine_empty_blocks(num_blocks, &mut rpc_client);
    }
}

pub fn mine_block(rpc_client: &mut dyn RpcClient, txs: Vec<Transaction>) -> (u32, TxId) {
    let block_template = rpc_client.get_block_template().unwrap();
    let block_height = block_template.height;

    let mut block_proposal = template_into_proposal(block_template, txs);
    let coinbase_txid = block_proposal.transactions.first().unwrap().txid();

    rpc_client.submit_block(block_proposal).unwrap();

    // TODO store coinbase txid in wallet

    (block_height, coinbase_txid)
}


pub fn mine_empty_blocks(num_blocks: u32, mut rpc_client: &mut dyn RpcClient) -> (u32, TxId) {

    if num_blocks <= 0 { panic!("num_blocks must be greater than 0") }

    let (block_height, coinbase_txid) = mine_block(rpc_client, vec![]);

    for _ in 1..num_blocks {
        mine_block(rpc_client, vec![]);
    };

    (block_height, coinbase_txid)
}