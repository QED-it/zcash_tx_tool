//! `assets` — the wallet's ZSA asset registry.

use abscissa_core::{Command, Runnable};

use crate::commands::wallet::{exit_err, WalletCtx};
use crate::components::asset_registry;

/// List known ZSA assets, or label a discovered asset
#[derive(clap::Parser, Command, Debug)]
pub struct AssetsCmd {
    /// AssetBase hex (or unique prefix) of an asset to label
    #[arg(long, requires = "name")]
    pub label: Option<String>,

    /// Human-readable name to attach to the asset given via --label
    #[arg(long, requires = "label")]
    pub name: Option<String>,
}

impl Runnable for AssetsCmd {
    fn run(&self) {
        let mut ctx = WalletCtx::load();

        if let (Some(asset_ref), Some(name)) = (&self.label, &self.name) {
            let asset = ctx.parse_asset(asset_ref);
            if asset_registry::find_by_asset(&mut ctx.conn, &asset).is_none() {
                exit_err(&format!("asset '{}' is not in the registry", asset_ref));
            }
            if let Err(e) = asset_registry::check_label(&mut ctx.conn, &asset, name) {
                exit_err(&e);
            }
            asset_registry::set_label(&mut ctx.conn, &asset, name);
            println!(
                "✔ labelled asset {} as '{}'",
                asset_registry::asset_base_hex(&asset),
                name
            );
            return;
        }

        let assets = asset_registry::list(&mut ctx.conn);
        if assets.is_empty() {
            println!("no ZSA assets known yet — `issue` one or `sync` to discover received assets");
            return;
        }

        println!(
            "\n{:<26}{:<8}{:<11}asset base (on-chain id)",
            "asset", "issuer", "supply"
        );
        println!("{}", "─".repeat(110));
        for info in &assets {
            let name = info.display_name();
            let own = if info.is_own() { "me" } else { "other" };
            let finalized = if info.is_finalized() {
                "finalized"
            } else {
                "open"
            };
            println!(
                "{:<26}{:<8}{:<11}{}",
                name.chars().take(24).collect::<String>(),
                own,
                finalized,
                info.asset_base,
            );
        }
        println!("{}", "─".repeat(110));
        println!();
    }
}
