//! `test` - happy e2e flow that issues, transfers and burns an asset

use abscissa_core::{Command, Runnable};
use orchard::keys::Scope::External;

use crate::commands::test_balances::{check_balances, print_balances, TestBalances};
use crate::components::rpc_client::mock::MockZcashNode;
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::sync_from_height;
use crate::components::transactions::{
    create_burn_transaction, create_issue_transaction, create_transfer_transaction, mine,
};
use crate::components::wallet::Wallet;
use crate::prelude::*;

/// Run the E2E test
#[derive(clap::Parser, Command, Debug)]
pub struct TestOrchardZSACmd {}

impl Runnable for TestOrchardZSACmd {
    /// Run the `test` subcommand.
    fn run(&self) {
        let config = APP.config();
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        let mut wallet = Wallet::new(&config.wallet.seed_phrase);

        let issuer = wallet.address_for_account(0, External);
        let alice = wallet.address_for_account(1, External);

        prepare_test(
            config.chain.v6_activation_height,
            &mut wallet,
            &mut rpc_client,
        );

        // --------------------- Issue asset ---------------------

        let issue_tx =
            create_issue_transaction(issuer, 1000, "WETH".as_bytes().to_vec(), &mut wallet);

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
        print_balances("=== Initial balances ===", balances);

        mine(&mut wallet, &mut rpc_client, Vec::from([issue_tx]), true);

        let balances = TestBalances::get_asset(asset, &mut wallet);
        print_balances("=== Balances after issue ===", balances);

        // --------------------- ZSA transfer ---------------------

        let amount_to_transfer_1: i64 = 3;

        let transfer_tx_1 = create_transfer_transaction(
            issuer,
            alice,
            amount_to_transfer_1 as u64,
            asset,
            &mut wallet,
        );
        mine(
            &mut wallet,
            &mut rpc_client,
            Vec::from([transfer_tx_1]),
            false,
        );

        let expected_delta = TestBalances::new(-amount_to_transfer_1, amount_to_transfer_1);
        check_balances(
            "=== Balances after transfer ===",
            asset,
            balances,
            expected_delta,
            &mut wallet,
        );

        // --------------------- Burn asset ---------------------

        let balances = TestBalances::get_asset(asset, &mut wallet);

        let amount_to_burn_issuer: i64 = 7;
        let amount_to_burn_alice: i64 = amount_to_transfer_1;

        let burn_tx_issuer =
            create_burn_transaction(issuer, amount_to_burn_issuer as u64, asset, &mut wallet);
        let burn_tx_alice =
            create_burn_transaction(alice, amount_to_burn_alice as u64, asset, &mut wallet);

        mine(
            &mut wallet,
            &mut rpc_client,
            Vec::from([burn_tx_issuer, burn_tx_alice]),
            false,
        );

        let expected_delta = TestBalances::new(-amount_to_burn_issuer, -amount_to_burn_alice);
        check_balances(
            "=== Balances after burning ===",
            asset,
            balances,
            expected_delta,
            &mut wallet,
        );
    }
}

fn prepare_test(target_height: u32, wallet: &mut Wallet, rpc_client: &mut ReqwestRpcClient) {
    wallet.reset();
    sync_from_height(target_height, wallet, rpc_client);
}
