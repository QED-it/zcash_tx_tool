use abscissa_core::{Command, Runnable};
use orchard::keys::Scope::External;
use zcash_primitives::consensus::{BlockHeight, TEST_NETWORK};
use zcash_primitives::memo::MemoBytes;
use zcash_primitives::transaction::{Transaction, TxId};
use zcash_primitives::transaction::builder::Builder;
use zcash_primitives::transaction::components::{Amount, transparent, TxOut};
use zcash_primitives::transaction::fees::zip317::{FeeError, FeeRule};
use zcash_proofs::prover::LocalTxProver;
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::rpc_client::RpcClient;
use crate::prelude::*;
use crate::components::wallet::Wallet;


/// `shield` subcommand
#[derive(clap::Parser, Command, Debug)]
pub struct ShieldCmd {
}

impl Runnable for ShieldCmd {
    fn run(&self) {
        let config = APP.config();

        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());
        let mut wallet = Wallet::new(&config.wallet.seed_phrase);

        // TODO send shielding transaction

    }
}

pub fn create_shield_coinbase_tx(coinbase_txid: TxId, wallet: &mut Wallet) -> Transaction {

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

    info!("TxId: {}", tx.txid());
    tx
}
