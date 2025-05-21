//! `test-orchard` - happy e2e flow for the native asset

use abscissa_core::{Command, Runnable};
use orchard::keys::Scope::External;
use orchard::note::AssetBase;
use zcash_primitives::transaction::TxId;

use crate::commands::test_balances::{
    check_balances, print_balances, expected_balances_after_transfer, TestBalances, TransferInfo,
    expected_balances_after_mine, InfoBatch,
};
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::{
    create_shield_coinbase_transaction, mine, mine_empty_blocks, sync_from_height,
};
use crate::components::user::User;
use crate::prelude::*;

/// Run the E2E test
#[derive(clap::Parser, Command, Debug)]
pub struct TestOrchardCmd {}

impl Runnable for TestOrchardCmd {
    /// Run the `test` subcommand.
    fn run(&self) {
        let config = APP.config();
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        let mut wallet = User::random(&config.wallet.miner_seed_phrase);

        wallet.reset();

        let num_users = 2;

        let miner_idx = 0;
        let alice_idx = 1;

        let miner_addr = wallet.address_for_account(miner_idx, External);

        let coinbase_txid = prepare_test(
            config.chain.nu5_activation_height,
            &mut wallet,
            &mut rpc_client,
        );

        let balances = TestBalances::get_native_balances(num_users, &mut wallet);
        print_balances("=== Initial balances ===", AssetBase::native(), &balances);

        // --------------------- Shield miner's reward ---------------------

        let shielding_tx =
            create_shield_coinbase_transaction(miner_addr, coinbase_txid, &mut wallet);
        mine(&mut wallet, &mut rpc_client, Vec::from([shielding_tx]));

        let expected_balances = expected_balances_after_mine(&balances, 0);
        check_balances(
            AssetBase::native(),
            &expected_balances,
            &mut wallet,
            num_users,
        );

        print_balances(
            "=== Balances after shielding ===",
            AssetBase::native(),
            &expected_balances,
        );

        // --------------------- Create transfer ---------------------

        let amount_to_transfer_1: u64 = 2;
        let balances = TestBalances::get_native_balances(num_users, &mut wallet);
        let transfer_info = TransferInfo::new(
            miner_idx,
            alice_idx,
            AssetBase::native(),
            amount_to_transfer_1,
        );
        let transfers = InfoBatch::new_singleton(transfer_info);

        let expected_balances = expected_balances_after_transfer(&balances, &transfers);

        let transfer_txns = transfers.to_txns(&mut wallet);

        mine(&mut wallet, &mut rpc_client, transfer_txns);

        check_balances(
            AssetBase::native(),
            &expected_balances,
            &mut wallet,
            num_users,
        );

        print_balances(
            "=== Balances after transfer ===",
            AssetBase::native(),
            &expected_balances,
        );
    }
}

fn prepare_test(target_height: u32, wallet: &mut User, rpc_client: &mut ReqwestRpcClient) -> TxId {
    sync_from_height(target_height, wallet, rpc_client);
    let activate = wallet.last_block_height().is_none();
    let (_, coinbase_txid) = mine_empty_blocks(100, rpc_client, activate); // coinbase maturity = 100
    coinbase_txid
}
