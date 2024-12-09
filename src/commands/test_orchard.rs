//! `test` - happy e2e flow that issues, transfers and burns an asset

use abscissa_core::{Command, Runnable};
use orchard::keys::Scope::External;
use orchard::note::AssetBase;
use zcash_primitives::transaction::TxId;

use crate::commands::test_balances::{check_balances, print_balances, TestBalances};
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::create_transfer_transaction;
use crate::components::transactions::mine;
use crate::components::transactions::{
    create_shield_coinbase_transaction, mine_empty_blocks, sync_from_height,
};
use crate::components::wallet::Wallet;
use crate::prelude::*;

/// Run the E2E test
#[derive(clap::Parser, Command, Debug)]
pub struct TestOrchardCmd {}

impl Runnable for TestOrchardCmd {
    /// Run the `test` subcommand.
    fn run(&self) {
        let config = APP.config();
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        let mut wallet = Wallet::random(&config.wallet.miner_seed_phrase);

        wallet.reset();

        let miner = wallet.address_for_account(0, External);
        let alice = wallet.address_for_account(1, External);

        let coinbase_txid = prepare_test(
            config.chain.nu5_activation_height,
            &mut wallet,
            &mut rpc_client,
        );

        let mut balances = TestBalances::get_zec(&mut wallet);
        print_balances("=== Initial balances ===", AssetBase::native(), balances);

        // --------------------- Shield miner's reward ---------------------

        let shielding_tx = create_shield_coinbase_transaction(miner, coinbase_txid, &mut wallet);
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
            miner,
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
    }
}

fn prepare_test(
    target_height: u32,
    wallet: &mut Wallet,
    rpc_client: &mut ReqwestRpcClient,
) -> TxId {
    sync_from_height(target_height, wallet, rpc_client);
    let activate = wallet.last_block_height().is_none();
    let (_, coinbase_txid) = mine_empty_blocks(100, rpc_client, activate); // coinbase maturity = 100
    coinbase_txid
}
