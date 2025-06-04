//! `test-orchard-zsa-final` - happy e2e flow that issues, transfers and finalizes an
//! asset, then attempts to issue the same asset again (unsuccessfully).

use abscissa_core::{Command, Runnable};
use orchard::issuance::compute_asset_desc_hash;
use orchard::keys::Scope::External;
use crate::commands::test_balances::{
    check_balances, print_balances, expected_balances_after_transfer, TestBalances, TransferInfo,
    TxiBatch,
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

        let issuer_idx = 0;
        let alice_idx = 1;

        let issuer_addr = wallet.address_for_account(issuer_idx, External);

        let asset_desc_hash = compute_asset_desc_hash(b"WETH").unwrap();
        prepare_test(
            config.chain.nu7_activation_height,
            &mut wallet,
            &mut rpc_client,
        );

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

        // --------------------- Finalization ---------------------

        let finalization_tx = create_finalization_transaction(asset_desc_hash, &mut wallet);
        mine(&mut wallet, &mut rpc_client, Vec::from([finalization_tx]));

        let (invalid_issue_tx, _) =
            create_issue_transaction(issuer_addr, 2000, asset_desc_hash, false, &mut wallet);
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
