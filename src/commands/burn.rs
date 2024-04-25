//! `burn` - burn assets

use std::convert::TryInto;
use abscissa_core::{Command, Runnable};
use orchard::note::AssetBase;
use orchard::value::NoteValue;
use zcash_primitives::consensus::{BlockHeight, TEST_NETWORK};
use zcash_primitives::memo::MemoBytes;
use zcash_primitives::transaction::builder::Builder;
use zcash_primitives::transaction::components::Amount;
use zcash_primitives::transaction::fees::zip317::{FeeError, FeeRule};
use zcash_proofs::prover::LocalTxProver;
use crate::components::rpc_client::mock::MockZcashNode;
use crate::components::rpc_client::RpcClient;
use crate::prelude::*;
use crate::components::wallet::Wallet;


/// `burn` subcommand
#[derive(clap::Parser, Command, Debug)]
pub struct BurnCmd {
    amount_to_transfer: u64,
    asset_hex: String,
}

impl Runnable for BurnCmd {
    /// Run the `burn` subcommand.
    fn run(&self) {
        let config = APP.config();

        let mut rpc = MockZcashNode::new();
        let mut wallet = Wallet::new();

        let asset = AssetBase::from_bytes(hex::decode(&self.asset_hex).unwrap().as_slice().try_into().unwrap()).unwrap();

        burn(42, asset, &mut wallet, &mut rpc);
    }
}

pub fn burn(amount: u64, asset: AssetBase, wallet: &mut Wallet, rpc: &mut dyn RpcClient) {
    info!("Burn {} zatoshi", amount);

    let ovk = wallet.orchard_ovk();

    let mut tx = Builder::new(TEST_NETWORK, /*wallet.last_block_height().unwrap()*/ BlockHeight::from_u32(1_842_421), wallet.orchard_anchor());

    // Add inputs
    let inputs = wallet.select_spendable_notes(amount);
    let total_inputs_amount = inputs.iter().fold(0, |acc, input| acc + input.note.value().inner());

    info!("Total inputs amount: {}, amount to burn: {}", total_inputs_amount, amount);

    inputs.into_iter().for_each(|input| tx.add_orchard_spend::<FeeError>(input.sk, input.note, input.merkle_path).unwrap());

    // Add main transfer output
    tx.add_burn::<FeeError>(asset, NoteValue::from_raw(amount)).unwrap();

    // Add change output
    let change_amount = total_inputs_amount - amount;
    let change_address = wallet.change_address();
    tx.add_orchard_output::<FeeError>(Some(ovk), change_address, change_amount, asset, MemoBytes::empty()).unwrap();

    let fee_rule = &FeeRule::non_standard(Amount::from_u64(0).unwrap(), 20, 150, 34).unwrap();
    let prover = LocalTxProver::with_default_location().unwrap();

    let (tx, _) = tx.build(&prover, fee_rule).unwrap();

    let tx_hash = rpc.send_transaction(tx).unwrap();

    info!("TxId: {}", tx_hash);
}