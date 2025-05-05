//! `test-orchard-zsa-final` - happy e2e flow that issues, transfers and finalizes an
//! asset, then attempts to issue the same asset again (unsuccessfully).

use abscissa_core::{Command, Runnable};
use orchard::keys::Scope::External;
use crate::commands::test_balances::{
    check_balances, print_balances, expected_balances_after_transfer, TestBalances, TransferInfo,
};
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::{
    create_issue_transaction, mine, create_finalization_transaction, sync_from_height,
};
use crate::components::user::User;
use crate::prelude::*;

/// Run the E2E test
#[derive(clap::Parser, Command, Debug)]
pub struct TestOrchardZSAFinalCmd {}

impl Runnable for TestOrchardZSAFinalCmd {
    /// Run the `test` subcommand.
    fn run(&self) {
        let config = APP.config();
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        let mut wallet = User::random(&config.wallet.miner_seed_phrase);

        wallet.reset();

        let num_users = 2;

        let issuer_idx: u32 = 0;
        let alice_idx: u32 = 1;

        let issuer_ad = wallet.address_for_account(issuer_idx, External);

        let asset_description = b"WETH".to_vec();
        prepare_test(
            config.chain.v6_activation_height,
            &mut wallet,
            &mut rpc_client,
        );

        // --------------------- Issue asset ---------------------

        let issue_tx = create_issue_transaction(
            issuer_ad,
            1000,
            asset_description.clone(),
            true,
            &mut wallet,
        );

        let asset = issue_tx
            .issue_bundle()
            .unwrap()
            .actions()
            .head
            .notes()
            .first()
            .unwrap()
            .asset();

        let balances = TestBalances::get_asset_balances(asset, num_users, &mut wallet);
        print_balances("=== Initial balances ===", asset, &balances);

        mine(&mut wallet, &mut rpc_client, Vec::from([issue_tx]));

        let balances = TestBalances::get_asset_balances(asset, num_users, &mut wallet);
        print_balances("=== Balances after issue ===", asset, &balances);

        // --------------------- ZSA transfer ---------------------

        let amount_to_transfer_1 = 3;
        let transfers = vec![TransferInfo::new(
            issuer_idx,
            alice_idx,
            amount_to_transfer_1,
        )];
        // Generate expected balances after transfer
        let expected_balances = expected_balances_after_transfer(&balances, &transfers);

        let transfer_txns = transfers
            .iter()
            .map(|info| info.create_transfer_txn(asset, &mut wallet))
            .collect();

        mine(&mut wallet, &mut rpc_client, transfer_txns);

        check_balances(asset, &expected_balances, &mut wallet, num_users);

        print_balances("=== Balances after transfer ===", asset, &expected_balances);

        // --------------------- Finalization ---------------------

        let finalization_tx =
            create_finalization_transaction(asset_description.clone(), &mut wallet);
        mine(&mut wallet, &mut rpc_client, Vec::from([finalization_tx]));

        let invalid_issue_tx =
            create_issue_transaction(issuer_ad, 2000, asset_description, false, &mut wallet);
        mine(&mut wallet, &mut rpc_client, Vec::from([invalid_issue_tx])); // TODO expect failure

        // The balances should not change since the transaction should have been rejected.
        let actual_balances = TestBalances::get_asset_balances(asset, num_users, &mut wallet);
        print_balances(
            "=== Balances after attempt to issue after finalization ===",
            asset,
            &actual_balances,
        );
        check_balances(asset, &expected_balances, &mut wallet, num_users);
    }
}

fn prepare_test(target_height: u32, wallet: &mut User, rpc_client: &mut ReqwestRpcClient) {
    sync_from_height(target_height, wallet, rpc_client);
}
