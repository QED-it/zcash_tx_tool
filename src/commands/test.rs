//! `test` - happy e2e flow that issues, transfers and burns an asset

use abscissa_core::{Command, Runnable};
use abscissa_core::config::Reader;
use orchard::keys::Scope::External;
use zcash_primitives::transaction::TxId;
use crate::components::transactions::{mine, mine_empty_blocks};
use crate::components::transactions::create_shield_coinbase_tx;
use crate::components::transactions::sync_from_height;
use crate::components::transactions::create_transfer_tx;
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::prelude::*;
use crate::components::wallet::Wallet;
use crate::config::AppConfig;


/// Run the E2E test
#[derive(clap::Parser, Command, Debug)]
pub struct TestCmd {
}

#[derive(Debug, Copy, Clone)]
struct TestBalances {
    miner: i64,
    alice: i64,
}

impl TestBalances {
    fn new(miner: i64, alice: i64) -> Self {
        TestBalances {
            miner,
            alice,
        }
    }

    fn get(wallet: &mut Wallet) -> TestBalances {

        let miner = wallet.address_for_account(0, External);
        let alice = wallet.address_for_account(1, External);

        TestBalances {
            miner: wallet.balance(miner) as i64,
            alice: wallet.balance(alice) as i64,
        }
    }

}

impl Runnable for TestCmd {
    /// Run the `test` subcommand.
    fn run(&self) {
        let config = APP.config();
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        let mut wallet = Wallet::new(&config.wallet.seed_phrase);

        let miner = wallet.address_for_account(0, External);
        let alice = wallet.address_for_account(1, External);

        let coinbase_txid = prepare_test(&config, &mut wallet, &mut rpc_client);

        let mut balances = TestBalances::get(&mut wallet);
        print_balances("=== Initial balances ===", balances);

        // --------------------- Shield miner's reward ---------------------

        let shielding_tx = create_shield_coinbase_tx(miner, coinbase_txid, &mut wallet);
        mine(&mut wallet, &mut rpc_client, Vec::from([ shielding_tx ]));

        let expected_delta = TestBalances::new(500_000_000 /*coinbase_reward*/, 0);
        balances = check_balances("=== Balances after shielding ===", balances, expected_delta, &mut wallet);

        // --------------------- Create transfer ---------------------

        let amount_to_transfer_1: i64 = 2;

        let transfer_tx_1 = create_transfer_tx(miner, alice, amount_to_transfer_1 as u64, &mut wallet);
        mine(&mut wallet, &mut rpc_client, Vec::from([ transfer_tx_1 ]));

        let expected_delta = TestBalances::new(-amount_to_transfer_1, amount_to_transfer_1);
        check_balances("=== Balances after transfer ===", balances, expected_delta, &mut wallet);
    }
}

fn prepare_test(config: &Reader<AppConfig>, wallet: &mut Wallet, rpc_client: &mut ReqwestRpcClient) -> TxId {
    wallet.reset();
    sync_from_height(config.chain.nu5_activation_height, wallet, rpc_client);
    let (_, coinbase_txid) = mine_empty_blocks(100, rpc_client); // coinbase maturity = 100
    coinbase_txid
}

fn check_balances(header: &str, initial: TestBalances, expected_delta: TestBalances, wallet: &mut Wallet) -> TestBalances{
    let actual_balances = TestBalances::get(wallet);
    print_balances(header, actual_balances);
    assert_eq!(actual_balances.miner, initial.miner + expected_delta.miner);
    assert_eq!(actual_balances.alice, initial.alice + expected_delta.alice);
    actual_balances
}

fn print_balances(header: &str, balances: TestBalances) {
    info!("{}", header);
    info!("Miner's balance: {}", balances.miner);
    info!("Alice's balance: {}", balances.alice);
}