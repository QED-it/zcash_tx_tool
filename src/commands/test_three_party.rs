//! `test` - Scenario: Three parties, a manufacturer of prescription medicines,
//! a purchaser of the medicines, and a supplier of the medicines. The manufacturer issues a ZSA
//! for every dose of medicine produced. On purchase, the manufacturer transfers the corresponding
//! number of ZSAs to the purchaser. The purchaser then transfers the ZSAs to the supplier, in
//! exchange for the physical doses. The supplier burns the ZSAs after receiving them to signal the
//! sale of the medicines.

use abscissa_core::{Command, Runnable};
use orchard::keys::Scope::External;

use crate::commands::test_balances::{
    check_balances, print_balances, update_balances_after_burn, update_balances_after_transfer,
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

        let num_users = 3;

        let manufacturer_index = 0;
        let purchaser_index: u32 = 1;
        let supplier_index: u32 = 2;

        let manufacturer = wallet.address_for_account(manufacturer_index, External);

        let asset_description = b"MED".to_vec();
        prepare_test(
            config.chain.v6_activation_height,
            &mut wallet,
            &mut rpc_client,
        );

        // --------------------- Issue asset ---------------------

        let issue_tx = create_issue_transaction(
            manufacturer,
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

        let balances = TestBalances::get_asset(asset, num_users, &mut wallet);
        print_balances("=== Initial balances ===", asset, &balances);

        let current_height = wallet.last_block_height();
        mine(
            &mut wallet,
            &mut rpc_client,
            Vec::from([issue_tx]),
            current_height.is_none(),
        );

        let balances = TestBalances::get_asset(asset, num_users, &mut wallet);
        print_balances("=== Balances after issue ===", asset, &balances);

        // --------------------- ZSA transfer from manufacturer to purchaser ---------------------
        let amount_to_transfer_1 = 3;
        let transfer_info_vec = vec![TransferInfo::new(
            manufacturer_index,
            purchaser_index,
            amount_to_transfer_1,
        )];
        // Generate expected balances after transfer
        let expected_balances = update_balances_after_transfer(&balances, &transfer_info_vec);

        let transfer_tx_vec = transfer_info_vec
            .iter()
            .map(|info| info.create_transfer_txn(asset, &mut wallet))
            .collect();

        mine(&mut wallet, &mut rpc_client, transfer_tx_vec, false);

        check_balances(
            "=== Balances after transfer to purchaser ===",
            asset,
            expected_balances,
            &mut wallet,
            num_users,
        );

        // --------------------- ZSA transfer from purchaser to supplier ---------------------

        let balances = TestBalances::get_asset(asset, num_users, &mut wallet);
        let amount_to_transfer_2 = 1;

        let transfer_info_vec = vec![TransferInfo::new(
            purchaser_index,
            supplier_index,
            amount_to_transfer_2,
        )];

        // Generate expected balances after transfer
        let expected_balances = update_balances_after_transfer(&balances, &transfer_info_vec);

        let transfer_tx_vec = transfer_info_vec
            .iter()
            .map(|info| info.create_transfer_txn(asset, &mut wallet))
            .collect();

        mine(&mut wallet, &mut rpc_client, transfer_tx_vec, false);

        check_balances(
            "=== Balances after transfer to supplier ===",
            asset,
            expected_balances,
            &mut wallet,
            num_users,
        );

        // --------------------- Supplier burning asset ---------------------

        let balances = TestBalances::get_asset(asset, num_users, &mut wallet);
        let amount_to_burn_supplier = 1;

        let burn_vec = vec![BurnInfo::new(supplier_index, amount_to_burn_supplier)];

        // Generate expected balances after burn
        let expected_balances = update_balances_after_burn(&balances, &burn_vec);

        let burn_tx_vec = burn_vec
            .iter()
            .map(|info| info.create_burn_txn(asset, &mut wallet))
            .collect();

        mine(&mut wallet, &mut rpc_client, burn_tx_vec, false);

        // burn from issuer(account0) and alice(account1)
        check_balances(
            "=== Balances after burning by supplier ===",
            asset,
            expected_balances,
            &mut wallet,
            num_users,
        );
    }
}

fn prepare_test(target_height: u32, wallet: &mut User, rpc_client: &mut ReqwestRpcClient) {
    sync_from_height(target_height, wallet, rpc_client);
}
