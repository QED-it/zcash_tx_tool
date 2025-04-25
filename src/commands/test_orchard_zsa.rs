//! `test` - happy e2e flow that issues, transfers and burns an asset

use abscissa_core::{Command, Runnable};
use orchard::keys::Scope::External;
use orchard::note::AssetBase;
use zcash_primitives::transaction::TxId;
use crate::commands::test_balances::{check_balances, print_balances, TestBalances};
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::{create_shield_coinbase_transaction, create_swap_transaction, mine_empty_blocks, sync_from_height};
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

        let coinbase_txid = prepare_test(
            config.chain.nu5_activation_height,
            &mut wallet,
            &mut rpc_client,
        );

        let mut balances = TestBalances::get_zec(&mut wallet);
        print_balances("=== Initial balances ===", AssetBase::native(), balances);

        // --------------------- Shield miner's reward ---------------------

        let shielding_tx = create_shield_coinbase_transaction(issuer, coinbase_txid, &mut wallet);
        mine(
            &mut wallet,
            &mut rpc_client,
            Vec::from([shielding_tx]),
            false,
        );

        let expected_delta = TestBalances::new(625_000_000 /*coinbase_reward*/, 0);
        balances = check_balances(
            "=== Balances after shielding ===",
            AssetBase::native(),
            balances,
            expected_delta,
            &mut wallet,
        );

        // --------------------- Create transfer ---------------------

        let amount_to_transfer_1: i64 = 2;

        let transfer_tx_1 = create_transfer_transaction(
            issuer,
            alice,
            amount_to_transfer_1 as u64,
            AssetBase::native(),
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
            AssetBase::native(),
            balances,
            expected_delta,
            &mut wallet,
        );

        let asset_description = b"WETH".to_vec();

        // --------------------- Issue asset ---------------------

        let issue_tx =
            create_issue_transaction(issuer, 1000, asset_description.clone(), true, &mut wallet);

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

        let transfer_tx_2 = create_transfer_transaction(issuer, alice, amount_to_transfer_2, asset_2, &mut wallet);
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

        let expected_delta = TestBalances::new(amount_to_transfer_3 as i64, -(amount_to_transfer_3 as i64));
        check_balances(
            "=== Balances after transfer ===",
            asset,
            balances_3,
            expected_delta,
            &mut wallet,
        );
    }
}

pub fn prepare_test(target_height: u32, wallet: &mut User, rpc_client: &mut ReqwestRpcClient) -> TxId {
    sync_from_height(target_height, wallet, rpc_client);
    let activate = wallet.last_block_height().is_none();
    let (_, coinbase_txid) = mine_empty_blocks(100, rpc_client, activate); // coinbase maturity = 100
    coinbase_txid
}
