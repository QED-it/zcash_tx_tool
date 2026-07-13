//! First-class wallet subcommands: manage notes and balances, and perform
//! ZSA issuance, transfer, burn and finalization against a running node.
//!
//! These commands are thin CLI shells over the same underlying components
//! that the end-to-end test scenarios use (`components::wallet`,
//! `components::transactions`), so the wallet behaviour and the tested
//! behaviour are one and the same.

pub mod addresses;
pub mod assets;
pub mod balance;
pub mod burn;
pub mod finalize;
pub mod issue;
pub mod mine;
pub mod notes;
pub mod shield;
pub mod status;
pub mod sync;
pub mod transfer;

use diesel::SqliteConnection;
use orchard::keys::Scope::External;
use orchard::note::AssetBase;
use orchard::Address;
use zcash_address::unified::{self, Container, Encoding, Receiver};
use zcash_primitives::transaction::{Transaction, TxId};
use zcash_protocol::consensus::NetworkType;

use crate::components::asset_registry;
use crate::components::db;
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::rpc_client::RpcClient;
use crate::components::transactions::{mine_block, sync_from_height};
use crate::components::wallet::Wallet;
use crate::prelude::*;

/// Print an error message and terminate with a non-zero exit code.
pub fn exit_err(msg: &str) -> ! {
    eprintln!("error: {}", msg);
    std::process::exit(1);
}

/// Everything a wallet command needs: DB connection, RPC client and the
/// wallet itself, with keys for the configured accounts registered.
pub struct WalletCtx {
    pub conn: SqliteConnection,
    pub rpc: ReqwestRpcClient,
    pub wallet: Wallet,
    pub num_accounts: usize,
    pub nu7_height: u32,
    pub node_url: String,
}

impl WalletCtx {
    /// Load the wallet context from the application config.
    pub fn load() -> Self {
        let config = APP.config();
        let mut conn = match &config.wallet.db_path {
            Some(path) => db::establish_connection(path),
            None => db::open(),
        };
        let mut wallet = Wallet::new(&mut conn, &config.wallet.seed_phrase);
        wallet.register_accounts(config.wallet.num_accounts);
        let node_url = config.network.node_url();
        WalletCtx {
            conn,
            rpc: ReqwestRpcClient::new(node_url.clone()),
            wallet,
            num_accounts: config.wallet.num_accounts,
            nu7_height: config.chain.nu7_activation_height,
            node_url,
        }
    }

    /// Fail fast with a friendly message when the node is unreachable.
    pub fn require_node(&mut self) {
        if self.rpc.get_block_template().is_err() {
            exit_err(&format!(
                "cannot reach the Zcash node at {} — is Zebra running?",
                self.node_url
            ));
        }
    }

    /// Sync the wallet with the node and record newly discovered assets.
    pub fn sync(&mut self) {
        sync_from_height(
            &mut self.conn,
            self.nu7_height,
            &mut self.wallet,
            &mut self.rpc,
        );
        for asset in asset_registry::discover_assets_from_notes(&mut self.conn) {
            println!(
                "• discovered new asset in received notes: {}",
                asset_registry::asset_base_hex(&asset)
            );
        }
    }

    pub fn account_address(&mut self, account: usize) -> Address {
        self.wallet.address_for_account(account, External)
    }

    /// Parse a recipient: `account:<n>`, a unified address (`uregtest1…`),
    /// or an 86-char hex-encoded raw Orchard address.
    pub fn parse_recipient(&mut self, s: &str) -> Address {
        if let Some(idx) = s.strip_prefix("account:") {
            match idx.parse::<usize>() {
                Ok(i) => return self.wallet.address_for_account(i, External),
                Err(_) => exit_err(&format!("invalid account index in recipient '{}'", s)),
            }
        }
        if let Some(addr) = try_decode_unified_address(s) {
            return addr;
        }
        if s.len() == 86 {
            if let Ok(bytes) = hex::decode(s) {
                if let Ok(raw) = <[u8; 43]>::try_from(bytes.as_slice()) {
                    if let Some(addr) =
                        Option::<Address>::from(Address::from_raw_address_bytes(&raw))
                    {
                        return addr;
                    }
                }
            }
        }
        exit_err(&format!(
            "could not parse recipient '{}': expected `account:<n>`, a unified address, \
             or 86-char raw Orchard address hex",
            s
        ));
    }

    /// Parse an asset reference: `zec`/`native`, a registered asset
    /// description, or a (prefix of a) hex-encoded AssetBase.
    pub fn parse_asset(&mut self, s: &str) -> AssetBase {
        let lower = s.to_ascii_lowercase();
        if lower == "zec" || lower == "native" || lower == "zatoshi" {
            return AssetBase::zatoshi();
        }
        if let Some(info) = asset_registry::find_by_description(&mut self.conn, s) {
            if let Some(base) = info.asset() {
                return base;
            }
        }
        if s.len() >= 6 && s.chars().all(|c| c.is_ascii_hexdigit()) {
            let matches = asset_registry::find_by_base_prefix(&mut self.conn, &lower);
            match matches.len() {
                1 => {
                    if let Some(base) = matches[0].asset() {
                        return base;
                    }
                }
                n if n > 1 => exit_err(&format!(
                    "asset prefix '{}' is ambiguous ({} known assets match)",
                    s, n
                )),
                _ => {
                    if s.len() == 64 {
                        if let Ok(bytes) = hex::decode(&lower) {
                            if let Ok(raw) = <[u8; 32]>::try_from(bytes.as_slice()) {
                                if let Some(base) =
                                    Option::<AssetBase>::from(AssetBase::from_bytes(&raw))
                                {
                                    return base;
                                }
                            }
                        }
                    }
                }
            }
        }
        exit_err(&format!(
            "unknown asset '{}' — run `assets` to list known assets, or pass a \
             64-char AssetBase hex",
            s
        ));
    }

    /// Display name for an asset: registry description, ZEC, or short hex.
    pub fn asset_display(&mut self, asset: &AssetBase) -> String {
        if bool::from(asset.is_zatoshi()) {
            return "ZEC (zatoshis)".to_string();
        }
        match asset_registry::find_by_asset(&mut self.conn, asset) {
            Some(info) => info.display_name(),
            None => format!("[unknown {}…]", &asset_registry::asset_base_hex(asset)[..8]),
        }
    }

    /// Submit transactions: mine them into a block ourselves (default,
    /// regtest) or hand them to the node mempool (`--mempool`).
    pub fn submit(&mut self, txs: Vec<Transaction>, mempool: bool) {
        if mempool {
            for tx in txs {
                let txid = tx.txid();
                match self.rpc.send_transaction(tx) {
                    Ok(_) => println!("✔ transaction {} accepted by the node mempool", txid),
                    Err(e) => exit_err(&format!("node rejected transaction {}: {}", txid, e)),
                }
            }
            println!("  (run `mine` to produce a block including mempool transactions)");
        } else {
            let txids: Vec<TxId> = txs.iter().map(|tx| tx.txid()).collect();
            match mine_block(&mut self.rpc, txs) {
                Ok((height, _)) => {
                    for txid in txids {
                        println!("✔ transaction {} mined in block {}", txid, height);
                    }
                }
                Err(e) => exit_err(&format!("node rejected the block: {}", e)),
            }
            self.sync();
        }
    }

    /// Pretty-print a balance table: one row per known asset (native ZEC
    /// first), one column per account.
    pub fn print_balances(&mut self, header: &str) {
        let accounts: Vec<Address> = (0..self.num_accounts)
            .map(|i| self.wallet.address_for_account(i, External))
            .collect();

        let mut rows: Vec<(String, AssetBase)> =
            vec![("ZEC (zatoshis)".to_string(), AssetBase::zatoshi())];
        for info in asset_registry::list(&mut self.conn) {
            if let Some(base) = info.asset() {
                let marker = if info.is_finalized() { " ⊗" } else { "" };
                rows.push((format!("{}{}", info.display_name(), marker), base));
            }
        }

        println!("\n{}", header);
        print!("{:<34}", "asset");
        for i in 0..self.num_accounts {
            print!("{:>16}", format!("account {}", i));
        }
        println!();
        println!("{}", "─".repeat(34 + 16 * self.num_accounts));
        for (name, base) in rows {
            let display: String = if name.chars().count() > 32 {
                let truncated: String = name.chars().take(31).collect();
                format!("{}…", truncated)
            } else {
                name
            };
            print!("{:<34}", display);
            for addr in &accounts {
                let balance = self.wallet.balance(&mut self.conn, *addr, base);
                print!("{:>16}", balance);
            }
            println!();
        }
        println!("{}", "─".repeat(34 + 16 * self.num_accounts));
        println!("(⊗ = supply finalized)");
    }
}

/// Encode an Orchard address as a unified address for regtest.
pub fn encode_unified_address(addr: &Address) -> String {
    let ua = unified::Address::try_from_items(vec![Receiver::Orchard(addr.to_raw_address_bytes())])
        .expect("an Orchard receiver alone is a valid unified address");
    ua.encode(&NetworkType::Regtest)
}

/// Decode a unified address, extracting its Orchard receiver.
pub fn try_decode_unified_address(s: &str) -> Option<Address> {
    let (_net, ua) = unified::Address::decode(s).ok()?;
    ua.items().into_iter().find_map(|item| match item {
        Receiver::Orchard(bytes) => {
            Option::<Address>::from(Address::from_raw_address_bytes(&bytes))
        }
        _ => None,
    })
}
