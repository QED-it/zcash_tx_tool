//! End-to-end tests for main OrchardZSA asset operations: issue, transfer, and burn.
//!
//! This module verifies the key asset lifecycle flows for OrchardZSA, including:
//! - Issuing new assets
//! - Transferring assets between accounts
//! - Burning assets
//!
//! The tests ensure correct balance updates and transaction validity at each step.

use crate::commands::test_balances::{
    check_balances, expected_balances_after_transfer, print_balances, TestBalances, TransferInfo,
    TxiBatch,
};
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::{
    create_issue_transaction, create_swap_transaction_with_matcher, mine, sync_from_height,
};
use crate::components::user::User;
use crate::prelude::*;
use abscissa_core::{Command, Runnable};
use nonempty::NonEmpty;
use orchard::issuance::compute_asset_desc_hash;
use orchard::keys::Scope::External;

/// Run the E2E test
#[derive(clap::Parser, Command, Debug)]
pub struct TestAssetSwapsCmd {}

impl Runnable for TestAssetSwapsCmd {
    /// Run the `test` subcommand.
    fn run(&self) {
        let config = APP.config();
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        let mut blockchain_state = User::random(&config.wallet.miner_seed_phrase);

        blockchain_state.reset();

        let num_accounts = 2;

        let issuer = 0;
        let alice = 1;

        let issuer_addr = blockchain_state.address_for_account(issuer, External);
        let alice_addr = blockchain_state.address_for_account(alice, External);

        let asset_desc_hash = compute_asset_desc_hash(&NonEmpty::from_slice(b"WETH").unwrap());
        prepare_test(
            config.chain.nu7_activation_height,
            &mut blockchain_state,
            &mut rpc_client,
        );

        // --------------------- Issue asset ---------------------

        let (issue_tx, asset) = create_issue_transaction(
            issuer_addr,
            1000,
            asset_desc_hash,
            true,
            &mut blockchain_state,
        );

        print_balances(
            "=== Initial balances ===",
            asset,
            num_accounts,
            &mut blockchain_state,
        );

        mine(
            &mut blockchain_state,
            &mut rpc_client,
            Vec::from([issue_tx]),
        );

        print_balances(
            "=== Balances after issue ===",
            asset,
            num_accounts,
            &mut blockchain_state,
        );

        let matcher_index = 2;

        // Issue a new type of asset
        let asset_desc_hash_2 = compute_asset_desc_hash(&NonEmpty::from_slice(b"WBTC").unwrap());

        let (issue_tx_2, _) = create_issue_transaction(
            alice_addr,
            10,
            asset_desc_hash_2,
            true,
            &mut blockchain_state,
        );

        let asset_2 = issue_tx_2
            .issue_bundle()
            .unwrap()
            .actions()
            .head
            .notes()
            .first()
            .unwrap()
            .asset();

        mine(
            &mut blockchain_state,
            &mut rpc_client,
            Vec::from([issue_tx_2]),
        );

        let mut expected_balances_asset_1 =
            TestBalances::get_asset_balances(asset, num_accounts, &mut blockchain_state);
        let mut expected_balances_asset_2 =
            TestBalances::get_asset_balances(asset_2, num_accounts, &mut blockchain_state);

        let spread = 1;
        let swap_asset_a_amount = 10;
        let swap_asset_b_amount = 6;
        let swap_tx = create_swap_transaction_with_matcher(
            issuer,
            alice,
            matcher_index,
            swap_asset_a_amount,
            asset,
            swap_asset_b_amount,
            asset_2,
            spread,
            &mut blockchain_state,
        );

        expected_balances_asset_1.decrement(issuer, swap_asset_a_amount);
        expected_balances_asset_1.increment(alice, swap_asset_a_amount - spread);

        expected_balances_asset_2.decrement(alice, swap_asset_b_amount);
        expected_balances_asset_2.increment(issuer, swap_asset_b_amount - spread);

        mine(&mut blockchain_state, &mut rpc_client, Vec::from([swap_tx]));

        check_balances(
            asset,
            &expected_balances_asset_1,
            &mut blockchain_state,
            num_accounts,
        );

        print_balances(
            "=== Balances after swap for the first asset ===",
            asset,
            num_accounts,
            &mut blockchain_state,
        );

        check_balances(
            asset_2,
            &expected_balances_asset_2,
            &mut blockchain_state,
            num_accounts,
        );

        print_balances(
            "=== Balances after swap for the second asset ===",
            asset_2,
            num_accounts,
            &mut blockchain_state,
        );

        // --------------------- Use swapped notes ---------------------

        let amount_to_transfer_2 = 1;
        let transfer_info = TransferInfo::new(issuer, alice, asset_2, amount_to_transfer_2);
        let txi = TxiBatch::from_item(transfer_info);
        let expected_balances_asset_2 =
            expected_balances_after_transfer(&expected_balances_asset_2, &txi);
        let txns = txi.to_transactions(&mut blockchain_state);

        mine(&mut blockchain_state, &mut rpc_client, txns);

        check_balances(
            asset_2,
            &expected_balances_asset_2,
            &mut blockchain_state,
            num_accounts,
        );

        print_balances(
            "=== Balances after transfer ===",
            asset_2,
            num_accounts,
            &mut blockchain_state,
        );

        let balances = TestBalances::get_asset_balances(asset, num_accounts, &mut blockchain_state);
        let amount_to_transfer_3 = balances.0[alice as usize];
        print_balances(
            "=== Balances before transfer ===",
            asset,
            num_accounts,
            &mut blockchain_state,
        );

        let transfer_info = TransferInfo::new(alice, issuer, asset, amount_to_transfer_3);
        let txi = TxiBatch::from_item(transfer_info);
        let expected_balances = expected_balances_after_transfer(&expected_balances_asset_1, &txi);
        let txns = txi.to_transactions(&mut blockchain_state);

        mine(&mut blockchain_state, &mut rpc_client, txns);

        check_balances(
            asset,
            &expected_balances,
            &mut blockchain_state,
            num_accounts,
        );
        print_balances(
            "=== Balances after transfer ===",
            asset,
            num_accounts,
            &mut blockchain_state,
        );
    }
}

fn prepare_test(target_height: u32, wallet: &mut User, rpc_client: &mut ReqwestRpcClient) {
    sync_from_height(target_height, wallet, rpc_client);
}
