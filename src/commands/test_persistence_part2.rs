//! Persistence test, part 2: spend the asset issued by part 1.
//!
//! Recreates the wallet with the same fixed seed and, without resetting
//! wallet-local state, must successfully transfer the asset issued in
//! part 1. End-to-end proof that wallet_state, notes, and commitment-tree
//! marked positions were correctly persisted to and reloaded from disk.

use abscissa_core::{Command, Runnable};
use nonempty::NonEmpty;
use orchard::issuance::{auth::IssueValidatingKey, compute_asset_desc_hash};
use orchard::note::{AssetBase, AssetId};

use crate::commands::test_balances::{
    check_balances, expected_balances_after_transfer, print_balances, TestBalances, TransferInfo,
    TxiBatch,
};
use crate::components::db;
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::{mine, sync_from_height};
use crate::components::wallet::Wallet;
use crate::prelude::*;

#[derive(clap::Parser, Command, Debug)]
pub struct TestPersistencePart2Cmd {}

impl Runnable for TestPersistencePart2Cmd {
    fn run(&self) {
        let config = APP.config();
        let mut c = db::open();
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());

        // Same fixed seed as part 1. Wallet::new auto-loads the persisted
        // wallet_state row (the issued PERSIST asset's note position lives
        // there) so that the transfer below can witness it.
        let mut wallet = Wallet::new(&mut c, &config.wallet.seed_phrase);

        // Resume from the persisted head; usually a no-op here.
        sync_from_height(
            &mut c,
            config.chain.nu7_activation_height,
            &mut wallet,
            &mut rpc_client,
        );

        let issuer_idx = 0;
        let alice_idx = 1;
        let asset_desc_hash = compute_asset_desc_hash(&NonEmpty::from_slice(b"PERSIST").unwrap());
        // Reconstruct the same asset id as part 1 (deterministic from issuance key + desc hash).
        let asset = AssetBase::custom(&AssetId::new_v0(
            &IssueValidatingKey::from(&wallet.issuance_key()),
            &asset_desc_hash,
        ));

        let num_users = 2;
        let balances = TestBalances::get_asset_balances(&mut c, asset, num_users, &mut wallet);
        print_balances(
            "=== Persistence part 2: balances after reload ===",
            asset,
            &balances,
        );

        // Transfer 5 issuer → alice. Mining succeeds only if the wallet
        // correctly reconstructs merkle paths from the persisted tree.
        let transfer_info = TransferInfo::new(issuer_idx, alice_idx, asset, 5);
        let txi = TxiBatch::from_item(transfer_info);
        let expected_balances = expected_balances_after_transfer(&balances, &txi);
        let txs = txi.to_transactions(&mut c, &rpc_client, &mut wallet);
        mine(&mut c, &mut wallet, &mut rpc_client, txs).expect("transfer block mined successfully");

        check_balances(&mut c, asset, &expected_balances, &mut wallet, num_users);
        print_balances(
            "=== Persistence part 2: balances after transfer ===",
            asset,
            &expected_balances,
        );
    }
}
