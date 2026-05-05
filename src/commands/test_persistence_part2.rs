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
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::{mine, sync_from_height};
use crate::components::user::User;
use crate::prelude::*;

#[derive(clap::Parser, Command, Debug)]
pub struct TestPersistencePart2Cmd {}

impl Runnable for TestPersistencePart2Cmd {
    fn run(&self) {
        let config = APP.config();
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());

        // Same fixed seed as part 1; do NOT call wallet.reset() — we want the
        // persisted commitment tree, last_block_*, and notes restored from disk.
        let mut wallet = User::new(&config.wallet.seed_phrase, &config.wallet.miner_seed_phrase);

        // Resume from the persisted head; usually a no-op here.
        sync_from_height(
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
        let balances = TestBalances::get_asset_balances(asset, num_users, &mut wallet);
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
        let txs = txi.to_transactions(&rpc_client, &mut wallet);
        mine(&mut wallet, &mut rpc_client, txs).expect("transfer block mined successfully");

        check_balances(asset, &expected_balances, &mut wallet, num_users);
        print_balances(
            "=== Persistence part 2: balances after transfer ===",
            asset,
            &expected_balances,
        );
    }
}
