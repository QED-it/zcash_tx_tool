//! `test` - happy e2e flow that issues, transfers and burns an asset

use abscissa_core::{Command, Runnable};
use orchard::issuance::compute_asset_desc_hash;
use orchard::keys::Scope::External;

use crate::commands::test_balances::{check_balances, print_balances, TestBalances};
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::sync_from_height;
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

        let issuer = wallet.address_for_account(0, External);
        let alice = wallet.address_for_account(1, External);

        let asset_description = compute_asset_desc_hash(b"WETH").unwrap();
        prepare_test(
            config.chain.v6_activation_height,
            &mut wallet,
            &mut rpc_client,
        );

        // --------------------- Issue asset ---------------------

        let issue_tx = create_issue_transaction(issuer, 1000, asset_description, true, &mut wallet);

        let asset = issue_tx
            .issue_bundle()
            .unwrap()
            .actions()
            .head
            .notes()
            .first()
            .unwrap()
            .asset();

        let balances = TestBalances::get_asset(asset, &mut wallet);
        print_balances("=== Initial balances ===", asset, balances);

        let current_height = wallet.last_block_height();
        mine(
            &mut wallet,
            &mut rpc_client,
            Vec::from([issue_tx]),
            current_height.is_none(),
        );

        let balances = TestBalances::get_asset(asset, &mut wallet);
        print_balances("=== Balances after issue ===", asset, balances);

        // --------------------- ZSA transfer ---------------------

        let amount_to_transfer_1 = 3;

        let transfer_tx_1 =
            create_transfer_transaction(issuer, alice, amount_to_transfer_1, asset, &mut wallet);
        mine(
            &mut wallet,
            &mut rpc_client,
            Vec::from([transfer_tx_1]),
            false,
        );

        // transfer from issuer(account0) to alice(account1)
        let expected_delta =
            TestBalances::new(-(amount_to_transfer_1 as i64), amount_to_transfer_1 as i64);
        check_balances(
            "=== Balances after transfer ===",
            asset,
            balances,
            expected_delta,
            &mut wallet,
        );

        // --------------------- Burn asset ---------------------

        let balances = TestBalances::get_asset(asset, &mut wallet);

        let amount_to_burn_issuer = 7;
        let amount_to_burn_alice = amount_to_transfer_1 - 1;

        let burn_tx_issuer =
            create_burn_transaction(issuer, amount_to_burn_issuer, asset, &mut wallet);
        let burn_tx_alice =
            create_burn_transaction(alice, amount_to_burn_alice, asset, &mut wallet);

        mine(
            &mut wallet,
            &mut rpc_client,
            Vec::from([burn_tx_issuer, burn_tx_alice]),
            false,
        );

        // burn from issuer(account0) and alice(account1)
        let expected_delta = TestBalances::new(
            -(amount_to_burn_issuer as i64),
            -(amount_to_burn_alice as i64),
        );
        check_balances(
            "=== Balances after burning ===",
            asset,
            balances,
            expected_delta,
            &mut wallet,
        );

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
    }
}

fn prepare_test(target_height: u32, wallet: &mut User, rpc_client: &mut ReqwestRpcClient) {
    sync_from_height(target_height, wallet, rpc_client);
}
