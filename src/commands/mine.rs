//! `mine` - happy e2e flow that issues, transfers and burns an asset

use abscissa_core::{Command, Runnable};
use crate::commands::sync::{sync, sync_from_height};
use crate::components::rpc_client::mock::MockZcashNode;
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::rpc_client::{RpcClient, template_into_proposal};
use crate::prelude::*;
use crate::components::wallet::Wallet;


/// `mine` subcommand
#[derive(clap::Parser, Command, Debug)]
pub struct MineCmd {
}

impl Runnable for MineCmd {
    /// Run the `mine` subcommand.
    fn run(&self) {
        let config = APP.config();

        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        let mut wallet = Wallet::new(&config.wallet.seed_phrase);

        let block_template = rpc_client.get_block_template().unwrap();
        let target_height = block_template.height;

        let block_proposal = template_into_proposal(block_template, Vec::new());
        let block_hash = block_proposal.header.hash();

        rpc_client.submit_block(block_proposal).unwrap();

        sync_from_height(target_height, &mut wallet, &mut rpc_client);

        let best_block_hash = rpc_client.get_best_block_hash().unwrap();

        assert_eq!(best_block_hash, block_hash);

        let block = rpc_client.get_block(target_height).unwrap();

        let tx = rpc_client.get_transaction(block.tx_ids.first().unwrap(), &block_hash).unwrap();

        let transparent_out = &tx.transparent_bundle().unwrap().vout;

        // TODO check that transparent receiver is our address
    }
}