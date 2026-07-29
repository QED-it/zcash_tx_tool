//! `addresses` — show the wallet's shielded receiving addresses.

use abscissa_core::{Command, Runnable};

use crate::commands::wallet::{encode_unified_address, WalletCtx};

/// Show receiving addresses for the wallet accounts
#[derive(clap::Parser, Command, Debug)]
pub struct AddressesCmd {
    /// Number of accounts to show (defaults to the configured num_accounts)
    #[arg(long)]
    pub accounts: Option<usize>,
}

impl Runnable for AddressesCmd {
    fn run(&self) {
        let mut ctx = WalletCtx::load();
        let n = self.accounts.unwrap_or(ctx.num_accounts);

        println!("\nOrchard (ZSA-capable) receiving addresses");
        println!("─────────────────────────────────────────");
        for account in 0..n {
            let addr = ctx.account_address(account);
            println!("account {}:", account);
            println!("  unified : {}", encode_unified_address(&addr));
            println!("  raw hex : {}", hex::encode(addr.to_raw_address_bytes()));
        }
        if n > ctx.num_accounts {
            println!(
                "\nnote: sync only scans accounts 0..{}, so notes sent to accounts {}..{} \
                 stay invisible until `num_accounts` is raised in the config",
                ctx.num_accounts, ctx.num_accounts, n
            );
        }
        println!();
    }
}
