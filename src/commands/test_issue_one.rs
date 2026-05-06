//! Simple test for issuing a single ZSA asset.
//!
//! This module provides a minimal test scenario that only issues 1 asset,
//! making it useful for quick testing of the asset issuance functionality.

use abscissa_core::{Command, Runnable};
use nonempty::NonEmpty;
use orchard::issuance::compute_asset_desc_hash;
use orchard::keys::Scope::External;
use crate::commands::test_balances::{print_balances, TestBalances};
use crate::components::db;
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
        let mut c = db::open();
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        // Stable wallet identity so tree state and notes persist across runs.
        let mut wallet = User::new(
            &mut c,
            &config.wallet.seed_phrase,
            &config.wallet.miner_seed_phrase,
        );

        let num_users = 1;
        let issuer_idx = 0;
        let issuer_addr = wallet.address_for_account(issuer_idx, External);
        // Random per-run asset desc; distinct prefix from other test commands so a
        // shared-seed CLI user can run multiple tests in sequence without collision.
        let asset_desc = format!("ONE-{:016x}", rand::random::<u64>());
        let asset_desc_hash =
            compute_asset_desc_hash(&NonEmpty::from_slice(asset_desc.as_bytes()).unwrap());
        sync_from_height(
            &mut c,
            config.chain.nu7_activation_height,
            &mut wallet,
            &mut rpc_client,
        );

        // --------------------- Issue asset ---------------------

        let (issue_tx, asset) = create_issue_transaction(
            issuer_addr,
            1,
            asset_desc_hash,
            true,
            &rpc_client,
            &mut wallet,
        );
        let balances = TestBalances::get_asset_balances(&mut c, asset, num_users, &mut wallet);
        print_balances("=== Initial balances ===", asset, &balances);

        mine(&mut c, &mut wallet, &mut rpc_client, Vec::from([issue_tx]))
            .expect("block mined successfully");

        let balances = TestBalances::get_asset_balances(&mut c, asset, num_users, &mut wallet);
        print_balances("=== Balances after issue ===", asset, &balances);
    }
}
