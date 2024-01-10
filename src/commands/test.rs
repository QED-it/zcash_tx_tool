//! `test` - happy e2e flow that issues, transfers and burns an asset

use abscissa_core::{Command, Runnable};
use orchard::keys::IssuanceValidatingKey;
use orchard::keys::Scope::External;
use orchard::note::AssetBase;
use orchard::value::NoteValue;
use zcash_primitives::consensus::{BlockHeight, TEST_NETWORK};
use zcash_primitives::memo::MemoBytes;
use zcash_primitives::transaction::builder::Builder;
use zcash_primitives::transaction::components::{Amount, transparent, TxOut};
use zcash_primitives::transaction::fees::zip317::{FeeError, FeeRule};
use zcash_primitives::transaction::TxId;
use zcash_proofs::prover::LocalTxProver;
use crate::commands::burn::burn;
use crate::commands::issue::issue;
use crate::commands::sync::{sync, sync_from_height};
use crate::commands::transfer::transfer;
use crate::components::rpc_client::mock::MockZcashNode;
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

        let mut rpc_client = ReqwestRpcClient::new();
        let mut wallet = Wallet::new();

        let block_template = rpc_client.get_block_template().unwrap();

        let block_proposal = template_into_proposal(block_template);
        let coinbase_txid = block_proposal.transactions.first().unwrap().txid();

        rpc_client.submit_block(block_proposal).unwrap();

        shield_coinbase(coinbase_txid, &mut wallet, &mut rpc_client);

        // let amount = 42;
        //
        // let orchard_recipient = wallet.address_for_account(0, External);
        //
        // let asset_descr = "ZSA".to_string();
        //
        // let ivk = IssuanceValidatingKey::from(&wallet.issuance_key());
        // let asset: AssetBase = AssetBase::derive(&ivk, asset_descr.as_ref());
        //
        // issue(orchard_recipient, amount, asset_descr, &mut wallet, &mut rpc_client);
        //
        // sync_from_height(1060756, &mut wallet, &mut rpc_client);
        //
        // transfer(orchard_recipient, amount, asset, &mut wallet, &mut rpc_client);
        //
        // sync(&mut wallet, &mut rpc_client);
        //
        // burn(amount, asset, &mut wallet, &mut rpc_client);
        //
        // sync(&mut wallet, &mut rpc_client);
    }
}

pub fn shield_coinbase(txid: TxId, wallet: &mut Wallet, rpc: &mut dyn RpcClient) {

    info!("Shielding coinbase output from tx {}", txid);

    let mut tx = Builder::new(TEST_NETWORK, /*wallet.last_block_height().unwrap()*/ BlockHeight::from_u32(1_842_421), wallet.orchard_anchor());

    let coinbase_value = 50;
    let coinbase_amount = Amount::from_u64(coinbase_value).unwrap();
    let miner_taddr = wallet.miner_address();

    let sk = wallet.miner_sk();

    tx.add_transparent_input(sk, transparent::OutPoint::new(txid.0, 0), TxOut { value: coinbase_amount, script_pubkey: miner_taddr.script() }).unwrap();
    tx.add_orchard_output::<FeeError>(Some(wallet.orchard_ovk()), wallet.address_for_account(0, External), coinbase_value, AssetBase::native(), MemoBytes::empty()).unwrap();

    let fee_rule = &FeeRule::non_standard(Amount::from_u64(0).unwrap(), 20, 150, 34).unwrap();
    let prover = LocalTxProver::with_default_location().unwrap();
    let (tx, _) = tx.build(&prover, fee_rule).unwrap();

    let tx_hash = rpc.send_transaction(tx).unwrap();
    info!("TxId: {}", tx_hash);
}