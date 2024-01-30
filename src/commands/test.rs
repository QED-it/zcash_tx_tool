//! `test` - happy e2e flow that issues, transfers and burns an asset

use abscissa_core::{Command, Runnable};
use orchard::keys::Scope::External;
use zcash_primitives::consensus::{BlockHeight, TEST_NETWORK};
use zcash_primitives::memo::MemoBytes;
use zcash_primitives::transaction::builder::Builder;
use zcash_primitives::transaction::components::{Amount, transparent, TxOut};
use zcash_primitives::transaction::fees::zip317::{FeeError, FeeRule};
use zcash_primitives::transaction::{Transaction, TxId};
use zcash_proofs::prover::LocalTxProver;
use crate::commands::sync::sync_from_height;
use crate::commands::transfer::create_transfer_tx;
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::rpc_client::{RpcClient, template_into_proposal};
use crate::prelude::*;
use crate::components::wallet::Wallet;


/// `test` subcommand
#[derive(clap::Parser, Command, Debug)]
pub struct TestCmd {
}

impl Runnable for TestCmd {
    /// Run the `test` subcommand.
    fn run(&self) {
        let config = APP.config();

        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        let mut wallet = Wallet::new(&config.wallet.seed_phrase);

        wallet.reset(); // Delete all notes from DB

        sync_from_height(config.chain.nu5_activation_height, &mut wallet, &mut rpc_client);

        let (block_height, coinbase_txid) = mine_100_blocks(&mut rpc_client);

        let shielding_tx = create_shield_coinbase_tx(coinbase_txid, &mut wallet, &mut rpc_client);

        let (_, _) = mine_block(&mut rpc_client, Vec::from([shielding_tx]));
        sync_from_height(block_height, &mut wallet, &mut rpc_client);

        let transfer_tx = create_transfer_tx(wallet.address_for_account(0, External), 1, &mut wallet, &mut rpc_client);

        let (block_height, _) = mine_block(&mut rpc_client, Vec::from([transfer_tx]));
        sync_from_height(block_height, &mut wallet, &mut rpc_client);

        let transfer_tx_2 = create_transfer_tx(wallet.address_for_account(0, External), 2, &mut wallet, &mut rpc_client);

        let (block_height, _) = mine_block(&mut rpc_client, Vec::from([transfer_tx_2]));
        sync_from_height(block_height, &mut wallet, &mut rpc_client);
    }
}

pub fn create_shield_coinbase_tx(coinbase_txid: TxId, wallet: &mut Wallet, rpc: &mut dyn RpcClient) -> Transaction {

    info!("Shielding coinbase output from tx {}", coinbase_txid);

    let mut tx = Builder::new(TEST_NETWORK, /*wallet.last_block_height().unwrap()*/ BlockHeight::from_u32(1_842_421), wallet.orchard_anchor());

    let coinbase_value = 500000000;
    let coinbase_amount = Amount::from_u64(coinbase_value).unwrap();
    let miner_taddr = wallet.miner_address();

    let sk = wallet.miner_sk();

    tx.add_transparent_input(sk, transparent::OutPoint::new(coinbase_txid.0, 0), TxOut { value: coinbase_amount, script_pubkey: miner_taddr.script() }).unwrap();
    tx.add_orchard_output::<FeeError>(Some(wallet.orchard_ovk()), wallet.address_for_account(0, External), coinbase_value, MemoBytes::empty()).unwrap();

    let fee_rule = &FeeRule::non_standard(Amount::from_u64(0).unwrap(), 20, 150, 34).unwrap();
    let prover = LocalTxProver::with_default_location().unwrap();
    let (tx, _) = tx.build(&prover, fee_rule).unwrap();

    //let tx_hash = rpc.send_transaction(tx).unwrap();
    info!("TxId: {}", tx.txid());
    tx
}

pub fn mine_block(rpc_client: &mut dyn RpcClient, txs: Vec<Transaction>) -> (u32, TxId) {
    let block_template = rpc_client.get_block_template().unwrap();
    let block_height = block_template.height;

    let mut block_proposal = template_into_proposal(block_template, txs);
    let coinbase_txid = block_proposal.transactions.first().unwrap().txid();

    rpc_client.submit_block(block_proposal).unwrap();
    (block_height, coinbase_txid)
}

/// We need to mine 100 blocks to be able to spend the coinbase outputs (coinbase maturity = 100)
pub fn mine_100_blocks(rpc_client: &mut dyn RpcClient) -> (u32, TxId) {
    let (block_height, coinbase_txid) = mine_block(rpc_client, vec![]);

    for _ in 0..99 {
        mine_block(rpc_client, vec![]);
    }

    (block_height, coinbase_txid)
}