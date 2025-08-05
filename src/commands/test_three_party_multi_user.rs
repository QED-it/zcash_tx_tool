//! `test-three-party-multi-user` - Scenario: Three parties, a manufacturer of prescription medicines,
//! a purchaser of the medicines, and a supplier of the medicines. The manufacturer issues a ZSA
//! for every dose of medicine produced. On purchase, the manufacturer transfers the corresponding
//! number of ZSAs to the purchaser. The purchaser then transfers the ZSAs to the supplier, in
//! exchange for the physical doses. The supplier burns the ZSAs after receiving them to signal the
//! sale of the medicines.
//!
//! The tests ensure correct balance updates and transaction validity at each step of this scenario.

use abscissa_core::{Command, Runnable};
use orchard::issuance::compute_asset_desc_hash;
use orchard::keys::Scope::External;

use crate::commands::test_balances::{
    print_balances, expected_balances_after_burn, expected_balances_after_transfer, BurnInfo,
    TestBalances, TransferInfo, TxiBatch, check_balances_multi_user,
};
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::{
    create_issue_transaction, mine_and_sync_others, sync_from_height,
};
use crate::components::user::User;
use crate::prelude::*;

/// Run the test scenario
#[derive(clap::Parser, Command, Debug)]
pub struct TestThreePartyMultiUserCmd {}

impl Runnable for TestThreePartyMultiUserCmd {
    /// Run the `test` subcommand.
    fn run(&self) {
        let config = APP.config();
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());

        let num_users = 4; // Manufacturer, Purchaser, Supplier, Miner

        let mut manufacturer = User::random(&config.wallet.miner_seed_phrase);
        let mut purchaser = User::random(&config.wallet.miner_seed_phrase);
        let mut supplier = User::random(&config.wallet.miner_seed_phrase);
        let mut miner = User::random(&config.wallet.miner_seed_phrase);

        manufacturer.reset();
        purchaser.reset();
        supplier.reset();
        miner.reset();

        let manufacturer_addr = manufacturer.address_for_account(0, External);

        let mut wallets = [&mut manufacturer, &mut purchaser, &mut supplier, &mut miner];
        let manufacturer_idx = 0;
        let purchaser_idx = 1;
        let supplier_idx = 2;
        let miner_idx = 3;

        prepare_test(
            config.chain.nu7_activation_height,
            &mut wallets,
            &mut rpc_client,
        );

        // --------------------- Issue asset ---------------------

        let asset_desc_hash = compute_asset_desc_hash(b"MED").unwrap();

        let (issue_tx, asset) = create_issue_transaction(
            manufacturer_addr,
            1000,
            asset_desc_hash,
            true,
            wallets[manufacturer_idx],
        );

        let balances = TestBalances::get_asset_balances_multi_user(asset, num_users, &mut wallets);
        print_balances("=== Initial balances ===", asset, &balances);

        mine_and_sync_others(
            miner_idx,
            &mut wallets,
            &mut rpc_client,
            Vec::from([issue_tx]),
        );

        let balances = TestBalances::get_asset_balances_multi_user(asset, num_users, &mut wallets);
        print_balances("=== Balances after issue ===", asset, &balances);

        // --------------------- ZSA transfer from manufacturer to purchaser ---------------------
        let amount_to_transfer_1 = 3;
        let transfer_info =
            TransferInfo::new(manufacturer_idx, purchaser_idx, asset, amount_to_transfer_1);
        let txi = TxiBatch::from_item(transfer_info);

        let expected_balances = expected_balances_after_transfer(&balances, &txi);

        let txs = txi.to_transactions_multi_user(&mut wallets);

        mine_and_sync_others(miner_idx, &mut wallets, &mut rpc_client, txs);

        check_balances_multi_user(asset, &expected_balances, &mut wallets, num_users);

        print_balances(
            "=== Balances after transfer to purchaser ===",
            asset,
            &expected_balances,
        );

        // --------------------- ZSA transfer from purchaser to supplier ---------------------

        let balances = TestBalances::get_asset_balances_multi_user(asset, num_users, &mut wallets);
        let amount_to_transfer_2 = 1;

        let transfer_info =
            TransferInfo::new(purchaser_idx, supplier_idx, asset, amount_to_transfer_2);
        let txi = TxiBatch::from_item(transfer_info);

        // Generate expected balances after transfer
        let expected_balances = expected_balances_after_transfer(&balances, &txi);

        let txs = txi.to_transactions_multi_user(&mut wallets);

        mine_and_sync_others(miner_idx, &mut wallets, &mut rpc_client, txs);

        check_balances_multi_user(asset, &expected_balances, &mut wallets, num_users);

        print_balances(
            "=== Balances after transfer to supplier ===",
            asset,
            &expected_balances,
        );

        // --------------------- Supplier burning asset ---------------------

        let balances = TestBalances::get_asset_balances_multi_user(asset, num_users, &mut wallets);
        let amount_to_burn_supplier = 1;

        let txi = TxiBatch::from_item(BurnInfo::new(supplier_idx, asset, amount_to_burn_supplier));

        // Generate expected balances after burn
        let expected_balances = expected_balances_after_burn(&balances, &txi);

        let txs = txi.to_transactions_multi_user(&mut wallets);

        mine_and_sync_others(miner_idx, &mut wallets, &mut rpc_client, txs);

        check_balances_multi_user(asset, &expected_balances, &mut wallets, num_users);

        print_balances(
            "=== Balances after burning by supplier ===",
            asset,
            &expected_balances,
        );
    }
}

fn prepare_test(target_height: u32, wallets: &mut [&mut User], rpc_client: &mut ReqwestRpcClient) {
    for w in wallets {
        sync_from_height(target_height, w, rpc_client)
    }
}
