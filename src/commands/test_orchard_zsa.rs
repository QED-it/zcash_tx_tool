//! `test-orchard-zsa` - happy e2e flow that issues, transfers and burns an asset

use abscissa_core::{Command, Runnable};
use orchard::keys::Scope::External;
use crate::commands::test_balances::{
    check_balances, print_balances, expected_balances_after_burn, expected_balances_after_transfer,
    BurnInfo, TestBalances, TransferInfo, InfoBatch,
};
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::{create_issue_transaction, mine, sync_from_height};
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

        let asset_description = b"WETH".to_vec();
        prepare_test(
            config.chain.v6_activation_height,
            &mut wallet,
            &mut rpc_client,
        );

        // --------------------- Issue asset ---------------------

        let (issue_tx, asset) =
            create_issue_transaction(issuer_addr, 1000, &asset_description, true, &mut wallet);

        let balances = TestBalances::get_asset_balances(asset, num_users, &mut wallet);
        print_balances("=== Initial balances ===", asset, &balances);

        mine(&mut wallet, &mut rpc_client, Vec::from([issue_tx]));

        let balances = TestBalances::get_asset_balances(asset, num_users, &mut wallet);
        print_balances("=== Balances after issue ===", asset, &balances);

        // --------------------- ZSA transfer ---------------------

        let amount_to_transfer_1 = 3;
        let transfer_info = TransferInfo::new(issuer_idx, alice_idx, asset, amount_to_transfer_1);
        let transfers = InfoBatch::new_singleton(transfer_info);
        let expected_balances = expected_balances_after_transfer(&balances, &transfers);

        let transfer_txns = transfers.to_txns(&mut wallet);

        mine(&mut wallet, &mut rpc_client, transfer_txns);

        check_balances(asset, &expected_balances, &mut wallet, num_users);

        print_balances("=== Balances after transfer ===", asset, &expected_balances);

        // --------------------- Burn asset ---------------------

        let balances = TestBalances::get_asset_balances(asset, num_users, &mut wallet);

        let amount_to_burn_issuer = 7;
        let amount_to_burn_alice = amount_to_transfer_1 - 1;

        let mut burns = InfoBatch::<BurnInfo>::new_empty();
        burns.add_to_batch(BurnInfo::new(issuer_idx, asset, amount_to_burn_issuer));
        burns.add_to_batch(BurnInfo::new(alice_idx, asset, amount_to_burn_alice));

        // Generate expected balances after burn
        let expected_balances = expected_balances_after_burn(&balances, &burns);

        let burn_txns = burns.to_txns(&mut wallet);

        mine(&mut wallet, &mut rpc_client, burn_txns);

        // burn from issuer(account0) and alice(account1)
        check_balances(asset, &expected_balances, &mut wallet, num_users);

        print_balances("=== Balances after burning ===", asset, &expected_balances);
    }
}

fn prepare_test(target_height: u32, wallet: &mut User, rpc_client: &mut ReqwestRpcClient) {
    sync_from_height(target_height, wallet, rpc_client);
}
