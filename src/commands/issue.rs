//! `issue` - issue assets

use abscissa_core::{Command, Runnable};
use orchard::Address;
use orchard::keys::IssuanceValidatingKey;
use orchard::keys::Scope::External;
use orchard::note::AssetBase;
use orchard::value::NoteValue;
use zcash_client_backend::address::RecipientAddress;
use zcash_primitives::consensus::{BlockHeight, TEST_NETWORK};
use zcash_primitives::memo::MemoBytes;
use zcash_primitives::transaction::builder::Builder;
use zcash_primitives::transaction::components::Amount;
use zcash_primitives::transaction::fees::zip317::{FeeError, FeeRule};
use zcash_proofs::prover::LocalTxProver;
use crate::commands::sync::{sync, SyncCmd};
use crate::components::rpc_client::mock::MockZcashNode;
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::rpc_client::RpcClient;
use crate::prelude::*;
use crate::components::wallet::Wallet;
use crate::util::orchard_address_from_ua;


/// `issue` subcommand
#[derive(clap::Parser, Command, Debug)]
pub struct IssueCmd {
    amount_to_issue: u64,
    asset_desc: String,
    dest_address: Option<String>,
}

impl Runnable for IssueCmd {
    /// Run the `issue` subcommand.
    fn run(&self) {
        let config = APP.config();

        let mut rpc_client = MockZcashNode::new();
        let mut wallet = Wallet::new();

        sync(&mut wallet, &mut rpc_client);

        let orchard_recipient = match &self.dest_address {
            Some(address) => orchard_address_from_ua(address),
            None => wallet.address_for_account(0, External)
        };

        issue(orchard_recipient, self.amount_to_issue, self.asset_desc.clone(), &mut wallet, &mut rpc_client);
    }
}

pub fn issue(recipient: Address, amount: u64, asset_desc: String, wallet: &mut Wallet, rpc: &mut MockZcashNode) {
    info!("Issue {} asset", amount);

    let mut tx = Builder::new(TEST_NETWORK, /*wallet.last_block_height().unwrap()*/ BlockHeight::from_u32(1_842_421), wallet.orchard_anchor());

    tx.add_issuance::<FeeError>(wallet.issuance_key(), asset_desc, recipient, NoteValue::from_raw(amount)).unwrap();

    let fee_rule = &FeeRule::non_standard(Amount::from_u64(0).unwrap(), 20, 150, 34).unwrap();
    let prover = LocalTxProver::with_default_location().unwrap();
    let (tx, _) = tx.build(&prover, fee_rule).unwrap();

    let tx_hash = rpc.send_transaction(tx).unwrap();
    info!("TxId: {}", tx_hash);
}