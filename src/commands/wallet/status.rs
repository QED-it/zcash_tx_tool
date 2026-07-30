//! `status` — node and wallet status overview.

use abscissa_core::{Command, Runnable};

use crate::commands::wallet::WalletCtx;
use crate::components::asset_registry;
use crate::components::persistence::sqlite as notes_db;
use crate::components::rpc_client::RpcClient;

/// Show node connection, sync position and wallet summary
#[derive(clap::Parser, Command, Debug)]
pub struct StatusCmd {}

impl Runnable for StatusCmd {
    fn run(&self) {
        let mut ctx = WalletCtx::load();

        println!("\nZcash ZSA wallet — status");
        println!("─────────────────────────────────────────────");
        println!("node URL          : {}", ctx.node_url);

        match ctx.rpc.get_block_template() {
            Ok(template) => {
                println!("node reachable    : yes");
                println!("chain tip height  : {}", template.height.saturating_sub(1));
            }
            Err(_) => {
                println!("node reachable    : NO — is Zebra running?");
            }
        }

        match ctx.wallet.last_block_height() {
            Some(height) => println!("wallet synced to  : {}", u32::from(height)),
            None => println!("wallet synced to  : never synced"),
        }

        let unspent = notes_db::list_unspent_notes(&mut ctx.conn);
        let all = notes_db::list_all_notes(&mut ctx.conn);
        let assets = asset_registry::list(&mut ctx.conn);
        println!("accounts          : {}", ctx.num_accounts);
        println!(
            "notes             : {} unspent / {} total",
            unspent.len(),
            all.len()
        );
        println!("known ZSA assets  : {}", assets.len());
        println!("─────────────────────────────────────────────");
        println!("run `sync` to update, `balance` for balances\n");
    }
}
