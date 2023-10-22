//! `transfer` - transfer assets

use abscissa_core::{Command, Runnable};
use orchard::Address;
use zcash_client_backend::address::RecipientAddress;
use zcash_primitives::consensus::TEST_NETWORK;
use zcash_primitives::memo::MemoBytes;
use zcash_primitives::transaction::builder::Builder;
use zcash_primitives::transaction::fees::zip317::{FeeError, FeeRule};
use zcash_proofs::prover::LocalTxProver;
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::rpc_client::RpcClient;
use crate::prelude::*;
use crate::components::wallet::Wallet;


/// `transfer` subcommand
#[derive(clap::Parser, Command, Debug)]
pub struct TransferCmd {
    amount_to_transfer: u64,
    dest_address: String
}

impl Runnable for TransferCmd {
    /// Run the `transfer` subcommand.
    fn run(&self) {
        let config = APP.config();

        let rpc_client = ReqwestRpcClient::new();
        let wallet = Wallet::empty();

        info!("Transfer {} zatoshi to {}", self.amount_to_transfer, self.dest_address);

        let mut tx = Builder::new(TEST_NETWORK, wallet.last_block_height().unwrap(), wallet.orchard_anchor());

         // Add inputs
        let inputs = wallet.select_spendable_notes(self.amount_to_transfer);
        let total_inputs_amount = inputs.iter().fold(0, |acc, input| acc + input.note.value().inner());
        inputs.into_iter().for_each(|input| tx.add_orchard_spend::<FeeError>(input.sk, input.note, input.merkle_path).unwrap());

        let ovk = wallet.orchard_ovk();

        // TODO implement reasonable address parsing
        let orchard_recipient: Address = match RecipientAddress::decode(&TEST_NETWORK /* TODO take from config */, &self.dest_address.as_str()) {
            Some(RecipientAddress::Unified(ua)) => {
                ua.orchard().unwrap().clone()
            }
            Some(_) => {
                panic!(
                    "{} did not decode to a unified address value.",
                    &self.dest_address.as_str()
                );
            }
            None => {
                panic!(
                    "Failed to decode unified address from test vector: {}",
                    &self.dest_address.as_str()
                );
            }
        };

        // Add main transfer output
        tx.add_orchard_output::<FeeError>(ovk.clone(), orchard_recipient, self.amount_to_transfer, MemoBytes::empty()).unwrap();

        // Add change output
        let change_amount = total_inputs_amount - self.amount_to_transfer;
        let change_address = wallet.change_address();
        tx.add_orchard_output::<FeeError>(ovk, change_address, change_amount, MemoBytes::empty()).unwrap();

        let fee_rule = &FeeRule::standard();
        let prover = LocalTxProver::with_default_location().unwrap();

        let (tx, _) = tx.build(&prover, fee_rule).unwrap();

        let tx_hash = rpc_client.send_transaction(tx).unwrap();
    }
}