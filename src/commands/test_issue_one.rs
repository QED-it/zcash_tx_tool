//! Simple test for issuing a single ZSA asset.
//!
//! This module provides a minimal test scenario that only issues 1 asset,
//! making it useful for quick testing of the asset issuance functionality.

use abscissa_core::{Command, Runnable};
use nonempty::NonEmpty;
use orchard::issuance::compute_asset_desc_hash;
use orchard::keys::Scope::External;
use crate::commands::test_balances::{print_balances, TestBalances};
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::{create_issue_transaction, mine, sync_from_height};
use crate::components::user::User;
use crate::prelude::*;

/// Run the simple issue test
#[derive(clap::Parser, Command, Debug)]
pub struct TestIssueOneCmd {}

impl Runnable for TestIssueOneCmd {
    /// Run the `test-issue-one` subcommand.
    fn run(&self) {
        let config = APP.config();
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        // Use a unique wallet for each test run to avoid conflicts with cached blocks
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut wallet = User::random(&config.wallet.miner_seed_phrase, Some(timestamp));

        wallet.reset();
        let num_users = 1;
        let issuer_idx = 0;
        let issuer_addr = wallet.address_for_account(issuer_idx, External);
        let asset_desc_hash = compute_asset_desc_hash(&NonEmpty::from_slice(b"WETH").unwrap());
        prepare_test(
            config.chain.nu7_activation_height,
            &mut wallet,
            &mut rpc_client,
        );

        // --------------------- Issue asset ---------------------

        let (issue_tx, asset) =
            create_issue_transaction(issuer_addr, 1, asset_desc_hash, true, &mut wallet);
        let balances = TestBalances::get_asset_balances(asset, num_users, &mut wallet);
        print_balances("=== Initial balances ===", asset, &balances);

        mine(&mut wallet, &mut rpc_client, Vec::from([issue_tx]))
            .expect("block mined successfully");

        let balances = TestBalances::get_asset_balances(asset, num_users, &mut wallet);
        print_balances("=== Balances after issue ===", asset, &balances);
    }
}

fn prepare_test(target_height: u32, wallet: &mut User, rpc_client: &mut ReqwestRpcClient) {
    sync_from_height(target_height, wallet, rpc_client);
}
