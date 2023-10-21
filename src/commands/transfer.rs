//! `transfer` - transfer assets

use abscissa_core::{Command, Runnable};
use orchard::Address;
use orchard::builder::Builder;
use orchard::bundle::Flags;
use orchard::circuit::ProvingKey;
use orchard::value::NoteValue;
use zcash_client_backend::address::RecipientAddress;
use zcash_primitives::consensus::TEST_NETWORK;
use rand::rngs::OsRng;
use zebra_chain::block;
use zebra_chain::parameters::NetworkUpgrade;
use zebra_chain::transaction::{LockTime, Transaction};
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::rpc_client::RpcClient;
use crate::prelude::*;
use crate::components::wallet::Wallet;
use crate::util;


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

        let mut orchard_builder = Builder::new(Flags::from_parts(true, true), wallet.orchard_anchor().unwrap());
         // Add inputs
        let inputs = wallet.select_spendable_notes(self.amount_to_transfer);
        let total_inputs_amount = inputs.iter().fold(0, |acc, input| acc + input.note.value().inner());
        inputs.into_iter().for_each(|input| orchard_builder.add_spend((&input.sk).into(), input.note, input.merkle_path).unwrap());

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
        orchard_builder.add_recipient(ovk.clone(), orchard_recipient, NoteValue::from_raw(self.amount_to_transfer), None).unwrap();

        // Add change output
        let change_amount = total_inputs_amount - self.amount_to_transfer;
        let change_address = wallet.change_address();
        orchard_builder.add_recipient(ovk, change_address, NoteValue::from_raw(change_amount), None).unwrap();

        let mut rng = OsRng;
        let sighash = [0; 32]; // TODO
        let pk = ProvingKey::build(); // TODO Move someplace else?

        let bundle = orchard_builder.build(rng).unwrap().create_proof(&pk, &mut rng)
            .unwrap()
            .prepare(rng, sighash)
            .finalize()
            .unwrap();

        let tx = Transaction::V5 {
            network_upgrade: NetworkUpgrade::Nu5,
            lock_time: LockTime::min_lock_time_timestamp(),
            expiry_height: block::Height(0),
            inputs: Vec::new(),
            outputs: Vec::new(),
            sapling_shielded_data: None,
            orchard_shielded_data: Some(util::convert_orchard_bundle_to_shielded_data(bundle).unwrap()),
        };

        let tx_hash = rpc_client.send_raw_transaction(tx).unwrap();
    }
}