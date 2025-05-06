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
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::{
    create_shield_coinbase_transaction, create_swap_transaction, mine_empty_blocks,
    sync_from_height,
};
use crate::components::transactions::{
    create_burn_transaction, create_issue_transaction, create_transfer_transaction, mine,
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
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        let mut wallet = User::random(&config.wallet.miner_seed_phrase);

        wallet.reset();

        let num_users = 2;

        let issuer_idx = 0;
        let alice_idx = 1;

        let issuer_addr = wallet.address_for_account(issuer_idx, External);

        let asset_desc_hash = compute_asset_desc_hash(&NonEmpty::from_slice(b"WETH").unwrap());
        prepare_test(
            config.chain.nu7_activation_height,
            &mut wallet,
            &mut rpc_client,
            Vec::from([transfer_tx_1]),
            false,
        );

        let expected_delta = TestBalances::new(-amount_to_transfer_1, amount_to_transfer_1);
        check_balances(
            "=== Balances after transfer ===",
            AssetBase::native(),
            balances,
            expected_delta,
            &mut wallet,
        );

        let asset_description = b"WETH".to_vec();

        // --------------------- Issue asset ---------------------

        let (issue_tx, asset) =
            create_issue_transaction(issuer_addr, 1000, asset_desc_hash, true, &mut wallet);

        let balances = TestBalances::get_asset_balances(asset, num_users, &mut wallet);
        print_balances("=== Initial balances ===", asset, &balances);

        mine(&mut wallet, &mut rpc_client, Vec::from([issue_tx]));

        let balances = TestBalances::get_asset_balances(asset, num_users, &mut wallet);
        print_balances("=== Balances after issue ===", asset, &balances);

        // --------------------- ZSA transfer ---------------------

        let amount_to_transfer_1 = 3;
        let transfer_info = TransferInfo::new(issuer_idx, alice_idx, asset, amount_to_transfer_1);
        let txi = TxiBatch::from_item(transfer_info);
        let expected_balances = expected_balances_after_transfer(&balances, &txi);

        let txs = txi.to_transactions(&mut wallet);

        mine(&mut wallet, &mut rpc_client, txs);

        check_balances(asset, &expected_balances, &mut wallet, num_users);

        print_balances("=== Balances after transfer ===", asset, &expected_balances);

        // --------------------- Burn asset ---------------------

        let balances = TestBalances::get_asset_balances(asset, num_users, &mut wallet);

        let amount_to_burn_issuer = 7;
        let amount_to_burn_alice = amount_to_transfer_1 - 1;

        let mut txi = TxiBatch::<BurnInfo>::empty();
        txi.add_to_batch(BurnInfo::new(issuer_idx, asset, amount_to_burn_issuer));
        txi.add_to_batch(BurnInfo::new(alice_idx, asset, amount_to_burn_alice));

        // Generate expected balances after burn
        let expected_balances = expected_balances_after_burn(&balances, &txi);

        let txs = txi.to_transactions(&mut wallet);

        mine(&mut wallet, &mut rpc_client, txs);

        // burn from issuer(account0) and alice(account1)
        check_balances(asset, &expected_balances, &mut wallet, num_users);

        // --------------------- Finalization ---------------------
        // TODO - uncomment when finalization is implemented
        // let finalization_tx = create_finalization_transaction(asset_description.clone(), &mut user);
        // mine(
        //     &mut user,
        //     &mut rpc_client,
        //     Vec::from([finalization_tx]),
        //     false,
        // );
        //
        // let invalid_issue_tx = create_issue_transaction(issuer, 2000, asset_description, &mut user);
        // mine(
        //     &mut user,
        //     &mut rpc_client,
        //     Vec::from([invalid_issue_tx]),
        //     false,
        // ); // TODO expect failure
        //
        // panic!("Invalid issue transaction was accepted");

        // --------------------- Swap ---------------------

        // Issue a new type of asset
        let asset_description_2 = b"WBTC".to_vec();
        let issue_tx_2 =
            create_issue_transaction(alice, 5, asset_description_2.clone(), true, &mut wallet);

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
            &mut wallet,
            &mut rpc_client,
            Vec::from([issue_tx_2]),
            current_height.is_none(),
        );

        let balances = TestBalances::get_asset(asset, &mut wallet);
        let balances_2 = TestBalances::get_asset(asset_2, &mut wallet);

        let swap_tx = create_swap_transaction(issuer, alice, 10, asset, 5, asset_2, &mut wallet);

        mine(&mut wallet, &mut rpc_client, Vec::from([swap_tx]), false);

        let expected_delta = TestBalances::new(-10, 10);
        check_balances(
            "=== Balances after swap for the first asset ===",
            asset,
            balances,
            expected_delta,
            &mut wallet,
        );

        let expected_delta_2 = TestBalances::new(5, -5);
        check_balances(
            "=== Balances after swap for the second asset ===",
            asset_2,
            balances_2,
            expected_delta_2,
            &mut wallet,
        );

        // --------------------- Use swapped notes ---------------------

        let balances_2 = TestBalances::get_asset(asset_2, &mut wallet);
        let amount_to_transfer_2 = 1;
        print_balances("=== Balances before transfer ===", asset_2, balances_2);

        let transfer_tx_2 =
            create_transfer_transaction(issuer, alice, amount_to_transfer_2, asset_2, &mut wallet);
        mine(
            &mut wallet,
            &mut rpc_client,
            Vec::from([transfer_tx_2]),
            false,
        );

        let expected_delta = TestBalances::new(-(amount_to_transfer_2 as i64), amount_to_transfer_2 as i64);
        check_balances(
            "=== Balances after transfer ===",
            asset_2,
            balances_2,
            expected_delta,
            &mut wallet,
        );

        let balances_3 = TestBalances::get_asset(asset, &mut wallet);
        let amount_to_transfer_3 = balances_3.account1 as u64;
        print_balances("=== Balances before transfer ===", asset, balances_3);

        let transfer_tx_3 = create_transfer_transaction(alice, issuer, amount_to_transfer_3, asset, &mut wallet);
        mine(
            &mut wallet,
            &mut rpc_client,
            Vec::from([transfer_tx_3]),
            false,
        );

        let expected_delta =
            TestBalances::new(amount_to_transfer_3 as i64, -(amount_to_transfer_3 as i64));
        check_balances(
            "=== Balances after transfer ===",
            asset,
            balances_3,
            expected_delta,
            &mut wallet,
        );
    }
}

pub fn prepare_test(
    target_height: u32,
    wallet: &mut User,
    rpc_client: &mut ReqwestRpcClient,
) -> TxId {
    sync_from_height(target_height, wallet, rpc_client);
    let activate = wallet.last_block_height().is_none();
    let (_, coinbase_txid) = mine_empty_blocks(100, rpc_client, activate); // coinbase maturity = 100
    coinbase_txid
}
