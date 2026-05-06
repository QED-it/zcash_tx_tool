//! End-to-end tests for operations on the native ZEC asset.
//!
//! This module verifies operations on the native asset continue to work as expected.
//! The tests ensure correct balance updates and transaction validity at each step.

use abscissa_core::{Command, Runnable};
use orchard::keys::Scope::External;
use orchard::note::AssetBase;
use zcash_primitives::transaction::TxId;

use crate::commands::test_balances::{
    check_balances, print_balances, expected_balances_after_transfer, TestBalances, TransferInfo,
    expected_balances_after_mine, TxiBatch,
};
use crate::components::db;
use crate::components::miner::MinerKey;
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::{
    create_shield_coinbase_transaction, mine, mine_empty_blocks, sync_from_height,
};
use crate::components::wallet::Wallet;
use crate::prelude::*;
use diesel::SqliteConnection;

/// Run the E2E test
#[derive(clap::Parser, Command, Debug)]
pub struct TestOrchardCmd {}

impl Runnable for TestOrchardCmd {
    /// Run the `test` subcommand.
    fn run(&self) {
        let config = APP.config();
        let mut c = db::open();
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        // Stable wallet identity so tree state and notes persist across runs;
        // each run shields a fresh coinbase and balance assertions are computed
        // against the current (carried-forward) wallet balance.
        let mut wallet = Wallet::new(&mut c, &config.wallet.seed_phrase);
        let miner_key = MinerKey::new(&config.wallet.miner_seed_phrase);

        let num_users = 2;

        let miner_idx = 0;
        let alice_idx = 1;

        let miner_addr = wallet.address_for_account(miner_idx, External);

        let coinbase_txid = prepare_test(
            &mut c,
            config.chain.nu5_activation_height,
            &mut wallet,
            &mut rpc_client,
        );

        let balances = TestBalances::get_native_balances(&mut c, num_users, &mut wallet);
        print_balances("=== Initial balances ===", AssetBase::zatoshi(), &balances);

        // --------------------- Shield miner's reward ---------------------

        let shielding_tx = create_shield_coinbase_transaction(
            miner_addr,
            coinbase_txid,
            &rpc_client,
            &mut wallet,
            &miner_key,
        );
        mine(
            &mut c,
            &mut wallet,
            &mut rpc_client,
            Vec::from([shielding_tx]),
        )
        .expect("block mined successfully");

        let expected_balances = expected_balances_after_mine(&balances, 0);
        check_balances(
            &mut c,
            AssetBase::zatoshi(),
            &expected_balances,
            &mut wallet,
            num_users,
        );

        print_balances(
            "=== Balances after shielding ===",
            AssetBase::zatoshi(),
            &expected_balances,
        );

        // --------------------- Create transfer ---------------------

        let amount_to_transfer_1: u64 = 2;
        let balances = TestBalances::get_native_balances(&mut c, num_users, &mut wallet);
        let transfer_info = TransferInfo::new(
            miner_idx,
            alice_idx,
            AssetBase::zatoshi(),
            amount_to_transfer_1,
        );
        let txi = TxiBatch::from_item(transfer_info);

        let expected_balances = expected_balances_after_transfer(&balances, &txi);

        let txs = txi.to_transactions(&mut c, &rpc_client, &mut wallet);

        mine(&mut c, &mut wallet, &mut rpc_client, txs).expect("block mined successfully");

        check_balances(
            &mut c,
            AssetBase::zatoshi(),
            &expected_balances,
            &mut wallet,
            num_users,
        );

        print_balances(
            "=== Balances after transfer ===",
            AssetBase::zatoshi(),
            &expected_balances,
        );
    }
}

fn prepare_test(
    c: &mut SqliteConnection,
    target_height: u32,
    wallet: &mut Wallet,
    rpc_client: &mut ReqwestRpcClient,
) -> TxId {
    sync_from_height(c, target_height, wallet, rpc_client);
    let activate = wallet.last_block_height().is_none();
    let (_, coinbase_txid) =
        mine_empty_blocks(100, rpc_client, activate).expect("block mined successfully"); // coinbase maturity = 100
    coinbase_txid
}
