//! `finalize` — permanently close an asset's supply (no further issuance).

use abscissa_core::{Command, Runnable};

use crate::commands::wallet::{exit_err, WalletCtx};
use crate::components::asset_registry;
use crate::components::transactions::create_finalization_transaction;

/// Finalize a ZSA asset: permanently prevent further issuance
#[derive(clap::Parser, Command, Debug)]
pub struct FinalizeCmd {
    /// Asset to finalize: description or AssetBase hex (must be issued by this wallet)
    pub asset: String,

    /// Submit to the node mempool instead of self-mining a block (regtest)
    #[arg(long)]
    pub mempool: bool,
}

impl Runnable for FinalizeCmd {
    fn run(&self) {
        let mut ctx = WalletCtx::load();
        ctx.require_node();
        ctx.sync();

        let asset = ctx.parse_asset(&self.asset);
        let Some(info) = asset_registry::find_by_asset(&mut ctx.conn, &asset) else {
            exit_err(&format!("asset '{}' is not in the registry", self.asset));
        };
        if !info.is_own() {
            exit_err(&format!(
                "asset '{}' was not issued by this wallet — only the issuer can finalize it",
                self.asset
            ));
        }
        if info.is_finalized() {
            exit_err(&format!("asset '{}' is already finalized", self.asset));
        }
        let desc_hash_hex = info
            .desc_hash
            .as_ref()
            .unwrap_or_else(|| exit_err("registry entry is missing the description hash"));
        let desc_hash: [u8; 32] = hex::decode(desc_hash_hex)
            .ok()
            .and_then(|v| v.try_into().ok())
            .unwrap_or_else(|| exit_err("registry entry holds a corrupt description hash"));

        println!("finalizing asset '{}'", info.display_name());

        let tx = create_finalization_transaction(desc_hash, &ctx.rpc, &mut ctx.wallet);
        ctx.submit(Vec::from([tx]), self.mempool);

        if !self.mempool {
            asset_registry::set_finalized(&mut ctx.conn, &asset);
            println!(
                "✔ asset '{}' finalized — issuance is now permanently closed",
                info.display_name()
            );
        } else {
            println!(
                "note: registry will mark the asset finalized once the tx is mined and synced"
            );
        }
    }
}
