//! End-to-end demo for main ZSA operations: issue, transfer, burn, swaps.
//!
//! This module verifies the key asset lifecycle flows for OrchardZSA, including:
//! - Issuing new assets
//! - Transferring assets between accounts
//! - Burning assets
//! - Swapping assets between accounts
//!
//! The tests ensure correct balance updates and transaction validity at each step.

use crate::commands::test_balances::{
    check_balances, expected_balances_after_transfer, print_balances, TestBalances, TransferInfo,
    TxiBatch,
};
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::{
    create_finalization_transaction, create_issue_transaction,
    create_swap_transaction_with_matcher, mine, sync_from_height,
};
use crate::components::user::User;
use crate::prelude::*;
use abscissa_core::{Command, Runnable};
use nonempty::NonEmpty;
use orchard::issuance::compute_asset_desc_hash;
use orchard::keys::Scope::External;

/// Run the E2E test
#[derive(clap::Parser, Command, Debug)]
pub struct DemoCmd {}

fn wait_for_enter() {
    use std::io::{self, Write};
    print!("\n\n\n\nPress Enter to continue...");
    io::stdout().flush().unwrap();
    let _ = io::stdin().read_line(&mut String::new());
}

impl Runnable for DemoCmd {
    /// Run the `test` subcommand.
    fn run(&self) {
        let config = APP.config();
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        let mut blockchain_state = User::random(&config.wallet.miner_seed_phrase);

        blockchain_state.reset();

        let num_accounts = 3;

        let stable_issuer = 0;
        let nefertiti = 1;
        let sam = 2;

        let stable_issuer_addr = blockchain_state.address_for_account(stable_issuer, External);
        let nefertiti_addr = blockchain_state.address_for_account(nefertiti, External);

        let stable_asset_desc_hash =
            compute_asset_desc_hash(&NonEmpty::from_slice(b"ZUSD").unwrap());
        prepare_test(
            config.chain.nu7_activation_height,
            &mut blockchain_state,
            &mut rpc_client,
        );

        // --------------------- Issue stablecoin ---------------------

        let (stable_issue_tx, stable_asset) = create_issue_transaction(
            stable_issuer_addr,
            1000,
            stable_asset_desc_hash,
            true,
            &mut blockchain_state,
        );

        print_balances(
            "\n\n\n\n\n=== Initial balances ===",
            stable_asset,
            num_accounts,
            &mut blockchain_state,
        );

        wait_for_enter();

        mine(
            &mut blockchain_state,
            &mut rpc_client,
            Vec::from([stable_issue_tx]),
        )
        .expect("block mined successfully");

        print_balances(
            "\n\n\n\n\n=== Balances after first issue ===",
            stable_asset,
            num_accounts,
            &mut blockchain_state,
        );

        wait_for_enter();

        // --------------------- Transfer stablecoins to Sam ---------------------

        let stable_amount_to_transfer = 20;
        let transfer_info =
            TransferInfo::new(stable_issuer, sam, stable_asset, stable_amount_to_transfer);
        let txi = TxiBatch::from_item(transfer_info);
        let balances =
            TestBalances::get_asset_balances(stable_asset, num_accounts, &mut blockchain_state);
        let expected_balances = expected_balances_after_transfer(&balances, &txi);

        let txs = txi.to_transactions(&mut blockchain_state);

        mine(&mut blockchain_state, &mut rpc_client, txs).expect("block mined successfully");

        check_balances(
            stable_asset,
            &expected_balances,
            &mut blockchain_state,
            num_accounts,
        );

        print_balances(
            "\n\n\n\n\n=== Balances after transfer ===",
            stable_asset,
            num_accounts,
            &mut blockchain_state,
        );

        wait_for_enter();

        // --------------------- Issue Certificate ---------------------

        let cert_asset_desc_hash = compute_asset_desc_hash(&NonEmpty::from_slice(b"Doge").unwrap());

        let (cert_issue_tx, cert_asset) = create_issue_transaction(
            nefertiti_addr,
            1,
            cert_asset_desc_hash,
            true,
            &mut blockchain_state,
        );

        // --------------------- Finalize Cert ---------------------

        let cert_finalize_tx =
            create_finalization_transaction(cert_asset_desc_hash, &mut blockchain_state);

        mine(
            &mut blockchain_state,
            &mut rpc_client,
            Vec::from([cert_issue_tx, cert_finalize_tx]),
        )
        .expect("block mined successfully");

        print_balances(
            "\n\n\n\n\n=== Asset balances are ===",
            stable_asset,
            num_accounts,
            &mut blockchain_state,
        );

        print_balances("\n", cert_asset, num_accounts, &mut blockchain_state);

        wait_for_enter();

        let mut expected_balances_stable_asset =
            TestBalances::get_asset_balances(stable_asset, num_accounts, &mut blockchain_state);
        let mut expected_balances_cert_asset =
            TestBalances::get_asset_balances(cert_asset, num_accounts, &mut blockchain_state);

        // --------------------- Swap Stablecoins for Cert ---------------------

        let matcher_index = 2;

        let spread = 0;
        let swap_stable_asset_amount = 2;
        let swap_cert_asset_amount = 1;
        let swap_tx = create_swap_transaction_with_matcher(
            sam,
            nefertiti,
            matcher_index,
            swap_stable_asset_amount,
            stable_asset,
            swap_cert_asset_amount,
            cert_asset,
            spread,
            &mut blockchain_state,
        );

        expected_balances_stable_asset.decrement(sam, swap_stable_asset_amount);
        expected_balances_stable_asset.increment(nefertiti, swap_stable_asset_amount - spread);

        expected_balances_cert_asset.decrement(nefertiti, swap_cert_asset_amount);
        expected_balances_cert_asset.increment(sam, swap_cert_asset_amount - spread);

        mine(&mut blockchain_state, &mut rpc_client, Vec::from([swap_tx]))
            .expect("block mined successfully");

        check_balances(
            stable_asset,
            &expected_balances_stable_asset,
            &mut blockchain_state,
            num_accounts,
        );

        print_balances(
            "\n\n\n\n\n=== Asset balances after swap ===",
            stable_asset,
            num_accounts,
            &mut blockchain_state,
        );

        check_balances(
            cert_asset,
            &expected_balances_cert_asset,
            &mut blockchain_state,
            num_accounts,
        );

        print_balances("\n", cert_asset, num_accounts, &mut blockchain_state);
        print!("\n\n\n\n")
    }
}

fn prepare_test(target_height: u32, wallet: &mut User, rpc_client: &mut ReqwestRpcClient) {
    sync_from_height(target_height, wallet, rpc_client);
}
