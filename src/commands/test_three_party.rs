//! `test-three-party` - Scenario: Three parties, a manufacturer of prescription medicines,
//! a purchaser of the medicines, and a supplier of the medicines. The manufacturer issues a ZSA
//! for every dose of medicine produced. On purchase, the manufacturer transfers the corresponding
//! number of ZSAs to the purchaser. The purchaser then transfers the ZSAs to the supplier, in
//! exchange for the physical doses. The supplier burns the ZSAs after receiving them to signal the
//! sale of the medicines.
//!
//! The tests ensure correct balance updates and transaction validity at each step of this scenario.

use abscissa_core::{Command, Runnable};
use nonempty::NonEmpty;
use orchard::issuance::compute_asset_desc_hash;
use orchard::keys::Scope::External;

use crate::commands::test_balances::{
    check_balances, print_balances, expected_balances_after_burn, expected_balances_after_transfer,
    BurnInfo, TestBalances, TransferInfo, TxiBatch,
};
use crate::components::db;
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::{create_issue_transaction, mine, sync_from_height};
use crate::components::wallet::Wallet;
use crate::prelude::*;

/// Run the test scenario
#[derive(clap::Parser, Command, Debug)]
pub struct TestThreePartyCmd {}

impl Runnable for TestThreePartyCmd {
    /// Run the `test` subcommand.
    fn run(&self) {
        let config = APP.config();
        let mut c = db::open();
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        // Stable wallet identity so tree state and notes persist across runs.
        let mut wallet = Wallet::new(&mut c, &config.wallet.seed_phrase);

        sync_from_height(
            &mut c,
            config.chain.nu7_activation_height,
            &mut wallet,
            &mut rpc_client,
        );

        let num_users = 3;

        let manufacturer_idx = 0;
        let purchaser_idx = 1;
        let supplier_idx = 2;

        let manufacturer_addr = wallet.address_for_account(manufacturer_idx, External);

        // --------------------- Issue asset ---------------------

        // Random per-run asset desc so each invocation issues a fresh asset, even
        // across CLI users sharing a seed against the same testnet.
        let asset_desc = format!("MED-{:016x}", rand::random::<u64>());
        let asset_desc_hash =
            compute_asset_desc_hash(&NonEmpty::from_slice(asset_desc.as_bytes()).unwrap());

        let (issue_tx, asset) = create_issue_transaction(
            manufacturer_addr,
            1000,
            asset_desc_hash,
            true,
            &rpc_client,
            &mut wallet,
        );

        let balances = TestBalances::get_asset_balances(&mut c, asset, num_users, &mut wallet);
        print_balances("=== Initial balances ===", asset, &balances);

        mine(&mut c, &mut wallet, &mut rpc_client, Vec::from([issue_tx]))
            .expect("block mined successfully");

        let balances = TestBalances::get_asset_balances(&mut c, asset, num_users, &mut wallet);
        print_balances("=== Balances after issue ===", asset, &balances);

        // --------------------- ZSA transfer from manufacturer to purchaser ---------------------
        let amount_to_transfer_1 = 3;
        let transfer_info =
            TransferInfo::new(manufacturer_idx, purchaser_idx, asset, amount_to_transfer_1);
        let txi = TxiBatch::from_item(transfer_info);

        let expected_balances = expected_balances_after_transfer(&balances, &txi);

        let txs = txi.to_transactions(&mut c, &rpc_client, &mut wallet);

        mine(&mut c, &mut wallet, &mut rpc_client, txs).expect("block mined successfully");

        check_balances(&mut c, asset, &expected_balances, &mut wallet, num_users);

        print_balances(
            "=== Balances after transfer to purchaser ===",
            asset,
            &expected_balances,
        );

        // --------------------- ZSA transfer from purchaser to supplier ---------------------

        let balances = TestBalances::get_asset_balances(&mut c, asset, num_users, &mut wallet);
        let amount_to_transfer_2 = 1;

        let transfer_info =
            TransferInfo::new(purchaser_idx, supplier_idx, asset, amount_to_transfer_2);
        let txi = TxiBatch::from_item(transfer_info);

        // Generate expected balances after transfer
        let expected_balances = expected_balances_after_transfer(&balances, &txi);

        let txs = txi.to_transactions(&mut c, &rpc_client, &mut wallet);

        mine(&mut c, &mut wallet, &mut rpc_client, txs).expect("block mined successfully");

        check_balances(&mut c, asset, &expected_balances, &mut wallet, num_users);

        print_balances(
            "=== Balances after transfer to supplier ===",
            asset,
            &expected_balances,
        );

        // --------------------- Supplier burning asset ---------------------

        let balances = TestBalances::get_asset_balances(&mut c, asset, num_users, &mut wallet);
        let amount_to_burn_supplier = 1;

        let txi = TxiBatch::from_item(BurnInfo::new(supplier_idx, asset, amount_to_burn_supplier));

        // Generate expected balances after burn
        let expected_balances = expected_balances_after_burn(&balances, &txi);

        let txs = txi.to_transactions(&mut c, &rpc_client, &mut wallet);

        mine(&mut c, &mut wallet, &mut rpc_client, txs).expect("block mined successfully");

        check_balances(&mut c, asset, &expected_balances, &mut wallet, num_users);

        print_balances(
            "=== Balances after burning by supplier ===",
            asset,
            &expected_balances,
        );
    }
}
