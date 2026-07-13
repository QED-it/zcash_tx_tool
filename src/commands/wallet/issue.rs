//! `issue` — issue units of a ZSA asset (ZIP 227).
//!
//! The asset is identified by its description string together with this
//! wallet's issuance key: the first `issue` for a description creates the
//! asset, later ones increase its supply until it is finalized.

use abscissa_core::{Command, Runnable};
use nonempty::NonEmpty;
use orchard::issuance::compute_asset_desc_hash;

use crate::commands::wallet::{exit_err, WalletCtx};
use crate::components::asset_registry;
use crate::components::transactions::create_issue_transaction;

/// Issue a ZSA asset (create it or increase its supply)
#[derive(clap::Parser, Command, Debug)]
pub struct IssueCmd {
    /// Asset description: defines the asset together with the issuer key
    pub asset_desc: String,

    /// Number of asset units to issue
    pub amount: u64,

    /// Recipient of the issued units: `account:<n>`, unified address, or hex
    #[arg(long, default_value = "account:0")]
    pub to: String,

    /// Submit to the node mempool instead of self-mining a block (regtest)
    #[arg(long)]
    pub mempool: bool,
}

impl Runnable for IssueCmd {
    fn run(&self) {
        let mut ctx = WalletCtx::load();
        ctx.require_node();
        ctx.sync();

        if self.amount == 0 {
            exit_err("amount must be greater than zero");
        }
        let desc_bytes = self.asset_desc.as_bytes();
        if desc_bytes.is_empty() || desc_bytes.len() > 512 {
            exit_err("asset description must be between 1 and 512 bytes");
        }

        let desc_hash = compute_asset_desc_hash(&NonEmpty::from_slice(desc_bytes).unwrap());
        let asset = ctx.wallet.asset_base_from_desc_hash(&desc_hash);

        // First issuance of this asset from this wallet? (ZIP 227 requires
        // the first issuance to be marked, creating the asset's reference note.)
        let existing = asset_registry::find_by_asset(&mut ctx.conn, &asset);
        if let Some(info) = &existing {
            if info.is_finalized() {
                exit_err(&format!(
                    "asset '{}' has been finalized — no further issuance is possible",
                    self.asset_desc
                ));
            }
        }
        let first_issuance = existing.is_none_or(|info| !info.is_own());

        let recipient = ctx.parse_recipient(&self.to);

        println!(
            "issuing {} units of '{}' ({}issuance) to {}",
            self.amount,
            self.asset_desc,
            if first_issuance { "first " } else { "re-" },
            self.to,
        );

        let (tx, asset) = create_issue_transaction(
            recipient,
            self.amount,
            desc_hash,
            first_issuance,
            &ctx.rpc,
            &mut ctx.wallet,
        );

        ctx.submit(Vec::from([tx]), self.mempool);
        asset_registry::upsert_own_asset(&mut ctx.conn, &asset, &self.asset_desc, &desc_hash);

        println!("✔ asset base: {}", asset_registry::asset_base_hex(&asset));
        if !self.mempool {
            ctx.print_balances("Balances after issuance");
        }
    }
}
