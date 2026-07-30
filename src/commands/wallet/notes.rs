//! `notes` — list the wallet's shielded notes.

use abscissa_core::{Command, Runnable};
use std::collections::HashMap;

use crate::commands::wallet::WalletCtx;
use crate::components::persistence::sqlite as notes_db;
use orchard::note::AssetBase;

/// List the wallet's notes (unspent by default)
#[derive(clap::Parser, Command, Debug)]
pub struct NotesCmd {
    /// Include spent notes
    #[arg(long)]
    pub all: bool,
}

impl Runnable for NotesCmd {
    fn run(&self) {
        let mut ctx = WalletCtx::load();

        // Map raw addresses back to account indices for display.
        let mut accounts: HashMap<Vec<u8>, usize> = HashMap::new();
        for i in 0..ctx.num_accounts {
            let addr = ctx.account_address(i);
            accounts.insert(addr.to_raw_address_bytes().to_vec(), i);
        }

        let notes = if self.all {
            notes_db::list_all_notes(&mut ctx.conn)
        } else {
            notes_db::list_unspent_notes(&mut ctx.conn)
        };

        if notes.is_empty() {
            println!(
                "no {}notes — receive funds or run `sync`",
                if self.all { "" } else { "unspent " }
            );
            return;
        }

        println!(
            "\n{:<5}{:<10}{:>14}  {:<28}{:<12}from tx",
            "id", "account", "amount", "asset", "status"
        );
        println!("{}", "─".repeat(96));
        for note in notes {
            let account = accounts
                .get(&note.recipient_address)
                .map(|i| format!("{}", i))
                .unwrap_or_else(|| "other".to_string());
            let asset_bytes: Option<[u8; 32]> = note.asset.as_slice().try_into().ok();
            let asset_name = asset_bytes
                .and_then(|b| Option::<AssetBase>::from(AssetBase::from_bytes(&b)))
                .map(|base| ctx.asset_display(&base))
                .unwrap_or_else(|| "?".to_string());
            let status = if note.spend_tx_id.is_some() {
                "spent"
            } else {
                "unspent"
            };
            // Tx ids display in reversed byte order, like explorers do.
            let mut txid = note.tx_id.clone();
            txid.reverse();
            println!(
                "{:<5}{:<10}{:>14}  {:<28}{:<12}{}…",
                note.id,
                account,
                note.amount,
                asset_name.chars().take(26).collect::<String>(),
                status,
                &hex::encode(txid)[..16],
            );
        }
        println!("{}", "─".repeat(96));
        println!();
    }
}
