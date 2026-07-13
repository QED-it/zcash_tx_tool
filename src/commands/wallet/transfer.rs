//! `transfer` — send asset units (ZSA or ZEC) to a shielded address.

use abscissa_core::{Command, Runnable};

use crate::commands::wallet::{exit_err, WalletCtx};
use crate::components::transactions::create_transfer_transaction;

/// Transfer ZSA units (or ZEC) to a shielded recipient
#[derive(clap::Parser, Command, Debug)]
pub struct TransferCmd {
    /// Asset to send: description, AssetBase hex (or prefix), or `zec`
    pub asset: String,

    /// Number of units to send
    pub amount: u64,

    /// Recipient: `account:<n>`, unified address, or raw Orchard hex
    #[arg(long)]
    pub to: String,

    /// Account to send from
    #[arg(long, default_value_t = 0)]
    pub from_account: usize,

    /// Submit to the node mempool instead of self-mining a block (regtest)
    #[arg(long)]
    pub mempool: bool,
}

impl Runnable for TransferCmd {
    fn run(&self) {
        let mut ctx = WalletCtx::load();
        ctx.require_node();
        ctx.sync();

        if self.amount == 0 {
            exit_err("amount must be greater than zero");
        }

        let asset = ctx.parse_asset(&self.asset);
        let sender = ctx.account_address(self.from_account);
        let recipient = ctx.parse_recipient(&self.to);

        let spendable = ctx.wallet.balance(&mut ctx.conn, sender, asset);
        if spendable < self.amount {
            exit_err(&format!(
                "insufficient funds: account {} holds {} of '{}', tried to send {}",
                self.from_account,
                spendable,
                ctx.asset_display(&asset),
                self.amount
            ));
        }

        println!(
            "transferring {} of '{}' from account {} to {}",
            self.amount,
            ctx.asset_display(&asset),
            self.from_account,
            self.to,
        );

        let tx = create_transfer_transaction(
            &mut ctx.conn,
            sender,
            recipient,
            self.amount,
            asset,
            &ctx.rpc,
            &mut ctx.wallet,
        );

        ctx.submit(Vec::from([tx]), self.mempool);
        if !self.mempool {
            ctx.print_balances("Balances after transfer");
        }
    }
}
