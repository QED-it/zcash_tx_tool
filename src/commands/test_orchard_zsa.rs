//! End-to-end tests for main OrchardZSA asset operations: issue, transfer, and burn.
//!
//! This module verifies the key asset lifecycle flows for OrchardZSA, including:
//! - Issuing new assets
//! - Transferring assets between accounts
//! - Burning assets
//!
//! The tests ensure correct balance updates and transaction validity at each step.

use abscissa_core::{Command, Runnable};
use nonempty::NonEmpty;
use orchard::issuance::compute_asset_desc_hash;
use orchard::keys::Scope::External;
use crate::commands::test_balances::{
    check_balances, print_balances, expected_balances_after_burn, expected_balances_after_transfer,
    BurnInfo, TestBalances, TransferInfo, TxiBatch,
};
use crate::components::db;
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::{
    create_finalization_transaction, create_issue_transaction, mine, sync_from_height,
};
use crate::components::user::User;
use crate::prelude::*;

/// Run the E2E test
#[derive(clap::Parser, Command, Debug)]
pub struct TestOrchardZSACmd {}

impl Runnable for TestOrchardZSACmd {
    /// Run the `test` subcommand.
    fn run(&self) {
        let config = APP.config();
        let mut c = db::open();
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        // Stable wallet identity so tree state and notes persist across runs.
        let mut wallet = User::new(&mut c, &config.wallet.seed_phrase);

        let num_users = 2;

        let issuer_idx = 0;
        let alice_idx = 1;

        let issuer_addr = wallet.address_for_account(issuer_idx, External);

        // Random per-run asset desc so the lifecycle is fresh on each invocation, even when
        // the wallet (and chain) carry forward from a previous run, and even when multiple
        // CLI users with the same seed run against the same testnet.
        let asset_desc = format!("WETH-{:016x}", rand::random::<u64>());
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
            1000,
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

        // --------------------- ZSA transfer ---------------------

        let amount_to_transfer_1 = 3;
        let transfer_info = TransferInfo::new(issuer_idx, alice_idx, asset, amount_to_transfer_1);
        let txi = TxiBatch::from_item(transfer_info);
        let expected_balances = expected_balances_after_transfer(&balances, &txi);

        let txs = txi.to_transactions(&mut c, &rpc_client, &mut wallet);

        mine(&mut c, &mut wallet, &mut rpc_client, txs).expect("block mined successfully");

        check_balances(&mut c, asset, &expected_balances, &mut wallet, num_users);

        print_balances("=== Balances after transfer ===", asset, &expected_balances);

        // --------------------- Burn asset ---------------------

        let balances = TestBalances::get_asset_balances(&mut c, asset, num_users, &mut wallet);

        let amount_to_burn_issuer = 7;
        let amount_to_burn_alice = amount_to_transfer_1 - 1;

        let mut txi = TxiBatch::<BurnInfo>::empty();
        txi.add_to_batch(BurnInfo::new(issuer_idx, asset, amount_to_burn_issuer));
        txi.add_to_batch(BurnInfo::new(alice_idx, asset, amount_to_burn_alice));

        // Generate expected balances after burn
        let expected_balances = expected_balances_after_burn(&balances, &txi);

        let txs = txi.to_transactions(&mut c, &rpc_client, &mut wallet);

        mine(&mut c, &mut wallet, &mut rpc_client, txs).expect("block mined successfully");

        // burn from issuer(account0) and alice(account1)
        check_balances(&mut c, asset, &expected_balances, &mut wallet, num_users);

        print_balances("=== Balances after burning ===", asset, &expected_balances);

        // --------------------- Finalization ---------------------
        let finalization_tx =
            create_finalization_transaction(asset_desc_hash, &rpc_client, &mut wallet);
        mine(
            &mut c,
            &mut wallet,
            &mut rpc_client,
            Vec::from([finalization_tx]),
        )
        .expect("block mined successfully");

        let invalid_issue_tx = create_issue_transaction(
            issuer_addr,
            2000,
            asset_desc_hash,
            true,
            &rpc_client,
            &mut wallet,
        );
        let result = mine(
            &mut c,
            &mut wallet,
            &mut rpc_client,
            Vec::from([invalid_issue_tx.0]),
        );
        assert!(
            result.is_err(),
            "Issue transaction was unexpectedly accepted after asset finalization"
        );
    }
}
