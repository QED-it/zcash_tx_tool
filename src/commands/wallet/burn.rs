//! `burn` — destroy asset units, provably reducing the asset's supply.

use abscissa_core::{Command, Runnable};

use crate::commands::wallet::{exit_err, WalletCtx};
use crate::components::transactions::create_burn_transaction;

/// Burn ZSA units (provably destroy them)
#[derive(clap::Parser, Command, Debug)]
pub struct BurnCmd {
    /// Asset to burn: description or AssetBase hex (or prefix)
    pub asset: String,

    /// Number of units to burn
    pub amount: u64,

    /// Account to burn from
    #[arg(long, default_value_t = 0)]
    pub from_account: usize,

    /// Submit to the node mempool instead of self-mining a block (regtest)
    #[arg(long)]
    pub mempool: bool,
}

impl Runnable for BurnCmd {
    fn run(&self) {
        let mut ctx = WalletCtx::load();
        ctx.require_node();
        ctx.sync();

        if self.amount == 0 {
            exit_err("amount must be greater than zero");
        }

        let asset = ctx.parse_asset(&self.asset);
        if bool::from(asset.is_zatoshi()) {
            exit_err("the native asset (ZEC) cannot be burned");
        }
        let burner = ctx.account_address(self.from_account);

        let spendable = ctx.wallet.balance(&mut ctx.conn, burner, asset);
        if spendable < self.amount {
            exit_err(&format!(
                "insufficient funds: account {} holds {} of '{}', tried to burn {}",
                self.from_account,
                spendable,
                ctx.asset_display(&asset),
                self.amount
            ));
        }

        println!(
            "burning {} of '{}' from account {}",
            self.amount,
            ctx.asset_display(&asset),
            self.from_account,
        );

        let tx = create_burn_transaction(
            &mut ctx.conn,
            burner,
            self.amount,
            asset,
            &ctx.rpc,
            &mut ctx.wallet,
        );

        ctx.submit(Vec::from([tx]), self.mempool);
        if !self.mempool {
            ctx.print_balances("Balances after burn");
        }
    }
}
