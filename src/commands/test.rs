//! `test` - happy e2e flow that issues, transfers and burns an asset

use abscissa_core::{Command, Runnable};
use orchard::keys::IssuanceValidatingKey;
use orchard::keys::Scope::External;
use orchard::note::AssetBase;
use zcash_primitives::consensus::{BlockHeight, TEST_NETWORK};
use zcash_primitives::memo::MemoBytes;
use zcash_primitives::transaction::builder::Builder;
use zcash_primitives::transaction::components::Amount;
use zcash_primitives::transaction::fees::zip317::{FeeError, FeeRule};
use zcash_proofs::prover::LocalTxProver;
use crate::commands::burn::burn;
use crate::commands::issue::issue;
use crate::commands::sync::sync;
use crate::commands::transfer::transfer;
use crate::components::rpc_client::mock::MockZcashNode;
use crate::components::rpc_client::RpcClient;
use crate::prelude::*;
use crate::components::wallet::Wallet;
use crate::util::orchard_address_from_ua;


/// `test` subcommand
#[derive(clap::Parser, Command, Debug)]
pub struct TestCmd {
}

impl Runnable for TestCmd {
    /// Run the `test` subcommand.
    fn run(&self) {
        let config = APP.config();

        let mut rpc_client = MockZcashNode::new();
        let mut wallet = Wallet::new();

        let amount = 42;
        let orchard_recipient = wallet.address_for_account(0, External);

        let asset_descr = "ZSA".to_string();

        let ivk = IssuanceValidatingKey::from(&wallet.issuance_key());
        let asset: AssetBase = AssetBase::derive(&ivk, asset_descr.as_ref());

        wallet.reset();
        sync(&mut wallet, &mut rpc_client);
        issue(orchard_recipient, amount, asset_descr, &mut wallet, &mut rpc_client);
        sync(&mut wallet, &mut rpc_client);

        transfer(orchard_recipient, amount, asset, &mut wallet, &mut rpc_client);

        sync(&mut wallet, &mut rpc_client);

        burn(amount, asset, &mut wallet, &mut rpc_client);

        sync(&mut wallet, &mut rpc_client);
    }
}