//! `test` - happy e2e flow that issues, transfers and burns an asset

use abscissa_core::{Command, Runnable};
use orchard::keys::Scope::External;
use crate::commands::mine::{mine_block, mine_empty_blocks};
use crate::commands::shield::create_shield_coinbase_tx;
use crate::commands::sync::sync_from_height;
use crate::commands::transfer::create_transfer_tx;
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::prelude::*;
use crate::components::wallet::Wallet;


/// Run the E2E test
#[derive(clap::Parser, Command, Debug)]
pub struct TestCmd {
}

impl Runnable for TestCmd {
    /// Run the `test` subcommand.
    fn run(&self) {
        let config = APP.config();

        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        let mut wallet = Wallet::new(&config.wallet.seed_phrase);

        wallet.reset(); // Delete all notes from DB

        sync_from_height(config.chain.nu5_activation_height, &mut wallet, &mut rpc_client);

        let (block_height, coinbase_txid) = mine_empty_blocks(100, &mut rpc_client); // coinbase maturity = 100

        let shielding_tx = create_shield_coinbase_tx(coinbase_txid, &mut wallet);

        let (_, _) = mine_block(&mut rpc_client, Vec::from([shielding_tx]));
        sync_from_height(block_height, &mut wallet, &mut rpc_client);

        let transfer_tx = create_transfer_tx(wallet.address_for_account(0, External), 1, &mut wallet, &mut rpc_client);

        let (block_height, _) = mine_block(&mut rpc_client, Vec::from([transfer_tx]));
        sync_from_height(block_height, &mut wallet, &mut rpc_client);

        let transfer_tx_2 = create_transfer_tx(wallet.address_for_account(0, External), 2, &mut wallet, &mut rpc_client);

        let (block_height, _) = mine_block(&mut rpc_client, Vec::from([transfer_tx_2]));
        sync_from_height(block_height, &mut wallet, &mut rpc_client);
    }
}