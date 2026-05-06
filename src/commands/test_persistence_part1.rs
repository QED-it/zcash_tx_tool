//! Persistence test, part 1: issue an asset under a fixed wallet seed.
//!
//! Pairs with `test-persistence-part2`. Part 1 issues and exits; part 2
//! recreates the wallet with the same seed and must spend the issued
//! note. A successful spend in part 2 proves end-to-end persistence
//! (block_data, wallet_state, notes, and commitment-tree marked
//! positions all survive the process restart).

use abscissa_core::{Command, Runnable};
use nonempty::NonEmpty;
use orchard::issuance::compute_asset_desc_hash;
use orchard::keys::Scope::External;

use crate::commands::test_balances::{print_balances, TestBalances};
use crate::components::db;
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::transactions::{create_issue_transaction, mine, sync_from_height};
use crate::components::user::User;
use crate::prelude::*;

#[derive(clap::Parser, Command, Debug)]
pub struct TestPersistencePart1Cmd {}

impl Runnable for TestPersistencePart1Cmd {
    fn run(&self) {
        let config = APP.config();
        let mut c = db::open();
        let mut rpc_client = ReqwestRpcClient::new(config.network.node_url());

        // Fixed seed so part 2 can re-derive the same keys. User::new
        // auto-loads any persisted wallet_state — for part 1's first run on
        // a fresh volume there's nothing to load.
        let mut wallet = User::new(&mut c, &config.wallet.seed_phrase);

        sync_from_height(
            &mut c,
            config.chain.nu7_activation_height,
            &mut wallet,
            &mut rpc_client,
        );

        let issuer_idx = 0;
        let issuer_addr = wallet.address_for_account(issuer_idx, External);
        let asset_desc_hash = compute_asset_desc_hash(&NonEmpty::from_slice(b"PERSIST").unwrap());

        let (issue_tx, asset) = create_issue_transaction(
            issuer_addr,
            100,
            asset_desc_hash,
            true,
            &rpc_client,
            &mut wallet,
        );
        mine(&mut c, &mut wallet, &mut rpc_client, Vec::from([issue_tx]))
            .expect("issue block mined successfully");

        let balances = TestBalances::get_asset_balances(&mut c, asset, 1, &mut wallet);
        print_balances(
            "=== Persistence part 1: issued PERSIST ===",
            asset,
            &balances,
        );
    }
}
