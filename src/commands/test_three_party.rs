//! `test-three-party` - Scenario: Three parties, a manufacturer of prescription medicines,
//! a purchaser of the medicines, and a supplier of the medicines. The manufacturer issues a ZSA
//! for every dose of medicine produced. On purchase, the manufacturer transfers the corresponding
//! number of ZSAs to the purchaser. The purchaser then transfers the ZSAs to the supplier, in
//! exchange for the physical doses. The supplier burns the ZSAs after receiving them to signal the
//! sale of the medicines.

use abscissa_core::{Command, Runnable};
use orchard::keys::Scope::External;

use crate::commands::test_balances::{
    check_balances, print_balances, expected_balances_after_burn, expected_balances_after_transfer,
    BurnInfo, TestBalances, TransferInfo,
};
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::sync_from_height;
use crate::components::transactions::{create_issue_transaction, mine};
use crate::components::user::User;
use crate::prelude::*;

/// Run the test scenario
#[derive(clap::Parser, Command, Debug)]
pub struct TestThreePartyCmd {}

impl Runnable for TestThreePartyCmd {
    /// Run the `test` subcommand.
    fn run(&self) {
        let config = APP.config();
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        let mut wallet = User::random(&config.wallet.miner_seed_phrase);

        wallet.reset();
        prepare_test(
            config.chain.v6_activation_height,
            &mut wallet,
            &mut rpc_client,
        );

        let num_users = 3;

        let manufacturer_idx = 0;
        let purchaser_idx = 1;
        let supplier_idx = 2;

        let manufacturer_ad = wallet.address_for_account(manufacturer_idx, External);

        // --------------------- Issue asset ---------------------

        let asset_description = b"MED".to_vec();

        let issue_tx = create_issue_transaction(
            manufacturer_ad,
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

        // --------------------- ZSA transfer from manufacturer to purchaser ---------------------
        let amount_to_transfer_1 = 3;
        let transfers = vec![TransferInfo::new(
            manufacturer_idx,
            purchaser_idx,
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

        print_balances(
            "=== Balances after transfer to purchaser ===",
            asset,
            &expected_balances,
        );

        // --------------------- ZSA transfer from purchaser to supplier ---------------------

        let balances = TestBalances::get_asset_balances(asset, num_users, &mut wallet);
        let amount_to_transfer_2 = 1;

        let transfers = vec![TransferInfo::new(
            purchaser_idx,
            supplier_idx,
            amount_to_transfer_2,
        )];

        // Generate expected balances after transfer
        let expected_balances = expected_balances_after_transfer(&balances, &transfers);

        let transfer_txns = transfers
            .iter()
            .map(|info| info.create_transfer_txn(asset, &mut wallet))
            .collect();

        mine(&mut wallet, &mut rpc_client, transfer_txns);

        check_balances(asset, &expected_balances, &mut wallet, num_users);

        print_balances(
            "=== Balances after transfer to supplier ===",
            asset,
            &expected_balances,
        );

        // --------------------- Supplier burning asset ---------------------

        let balances = TestBalances::get_asset_balances(asset, num_users, &mut wallet);
        let amount_to_burn_supplier = 1;

        let burns = vec![BurnInfo::new(supplier_idx, asset, amount_to_burn_supplier)];

        // Generate expected balances after burn
        let expected_balances = expected_balances_after_burn(&balances, &burns);

        let burn_txns = burns
            .iter()
            .map(|info| info.create_burn_txn(&mut wallet))
            .collect();

        mine(&mut wallet, &mut rpc_client, burn_txns);

        // burn from issuer(account0) and alice(account1)
        check_balances(asset, &expected_balances, &mut wallet, num_users);

        print_balances(
            "=== Balances after burning by supplier ===",
            asset,
            &expected_balances,
        );
    }
}

fn prepare_test(target_height: u32, wallet: &mut User, rpc_client: &mut ReqwestRpcClient) {
    sync_from_height(target_height, wallet, rpc_client);
}
