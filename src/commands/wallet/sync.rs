//! `sync` — scan the chain for the wallet's notes.

use abscissa_core::{Command, Runnable};

use crate::commands::wallet::WalletCtx;

/// Sync the wallet with the node
#[derive(clap::Parser, Command, Debug)]
pub struct SyncCmd {}

impl Runnable for SyncCmd {
    fn run(&self) {
        let mut ctx = WalletCtx::load();
        ctx.require_node();
        ctx.sync();
        match ctx.wallet.last_block_height() {
            Some(height) => println!("✔ wallet synced to block {}", u32::from(height)),
            None => println!("✔ wallet synced (chain is empty)"),
        }
        ctx.print_balances("Balances");
    }
}
