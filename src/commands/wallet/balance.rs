//! `balance` — balances per asset and account.

use abscissa_core::{Command, Runnable};

use crate::commands::wallet::WalletCtx;

/// Show balances for every known asset and account
#[derive(clap::Parser, Command, Debug)]
pub struct BalanceCmd {
    /// Skip syncing with the node first
    #[arg(long)]
    pub no_sync: bool,
}

impl Runnable for BalanceCmd {
    fn run(&self) {
        let mut ctx = WalletCtx::load();
        if !self.no_sync {
            ctx.require_node();
            ctx.sync();
        }
        ctx.print_balances("Balances");
    }
}
