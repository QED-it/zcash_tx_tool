//! Application Subcommands
//!
//! This is where you specify the subcommands of your application.
//!
//! The default application comes with two subcommands:
//!
//! - `start`: launches the application
//! - `--version`: print application version
//!
//! See the `impl Configurable` below for how to specify the path to the
//! application's configuration file.

mod clean;
mod get_block_data;
mod test_balances;
mod test_issue_one;
mod test_orchard;
mod test_orchard_zsa;
mod test_persistence_part1;
mod test_persistence_part2;
mod test_three_party;
mod wallet;

use crate::commands::clean::CleanCmd;
use crate::commands::get_block_data::GetBlockDataCmd;
use crate::commands::test_issue_one::TestIssueOneCmd;
use crate::commands::test_orchard::TestOrchardCmd;
use crate::commands::test_orchard_zsa::TestOrchardZSACmd;
use crate::commands::test_persistence_part1::TestPersistencePart1Cmd;
use crate::commands::test_persistence_part2::TestPersistencePart2Cmd;
use crate::commands::test_three_party::TestThreePartyCmd;
use crate::commands::wallet::addresses::AddressesCmd;
use crate::commands::wallet::assets::AssetsCmd;
use crate::commands::wallet::balance::BalanceCmd;
use crate::commands::wallet::burn::BurnCmd;
use crate::commands::wallet::finalize::FinalizeCmd;
use crate::commands::wallet::issue::IssueCmd;
use crate::commands::wallet::mine::MineCmd;
use crate::commands::wallet::notes::NotesCmd;
use crate::commands::wallet::shield::ShieldCmd;
use crate::commands::wallet::status::StatusCmd;
use crate::commands::wallet::sync::SyncCmd;
use crate::commands::wallet::transfer::TransferCmd;
use crate::config::AppConfig;
use abscissa_core::{Command, Configurable, FrameworkError, Runnable};
use std::path::PathBuf;

/// Application Configuration Filename
pub const CONFIG_FILE: &str = "config.toml";

/// Application subcommands need to be listed in an enum.
#[derive(clap::Parser, Command, Debug, Runnable)]
pub enum AppCmd {
    // ---- Wallet commands ----
    /// Show node connection and wallet summary
    Status(StatusCmd),
    /// Sync the wallet with the node
    Sync(SyncCmd),
    /// Show receiving addresses
    Addresses(AddressesCmd),
    /// Show balances per asset and account
    Balance(BalanceCmd),
    /// List the wallet's notes
    Notes(NotesCmd),
    /// List or label known ZSA assets
    Assets(AssetsCmd),
    /// Issue a ZSA asset
    Issue(IssueCmd),
    /// Transfer ZSA units or ZEC
    Transfer(TransferCmd),
    /// Burn ZSA units
    Burn(BurnCmd),
    /// Finalize a ZSA asset (stop issuance forever)
    Finalize(FinalizeCmd),
    /// Mine regtest block(s), including mempool transactions
    Mine(MineCmd),
    /// Mine and shield a coinbase reward (regtest ZEC faucet)
    Shield(ShieldCmd),
    /// Reset all local wallet state
    Clean(CleanCmd),

    // ---- End-to-end test scenarios ----
    TestOrchard(TestOrchardCmd),
    TestOrchardZSA(TestOrchardZSACmd),
    TestThreeParty(TestThreePartyCmd),
    TestIssueOne(TestIssueOneCmd),
    TestPersistencePart1(TestPersistencePart1Cmd),
    TestPersistencePart2(TestPersistencePart2Cmd),
    GetBlockData(GetBlockDataCmd),
}

/// Entry point for the application. It needs to be a struct to allow using subcommands!
#[derive(clap::Parser, Command, Debug)]
#[command(author, about, version)]
pub struct EntryPoint {
    #[command(subcommand)]
    cmd: AppCmd,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Use the specified config file
    #[arg(short, long)]
    pub config: Option<String>,
}

impl Runnable for EntryPoint {
    fn run(&self) {
        self.cmd.run()
    }
}

/// This trait allows you to define how application configuration is loaded.
impl Configurable<AppConfig> for EntryPoint {
    /// Location of the configuration file
    fn config_path(&self) -> Option<PathBuf> {
        // Check if the config file exists, and if it does not, ignore it.
        // If you'd like for a missing configuration file to be a hard error
        // instead, always return `Some(CONFIG_FILE)` here.
        let filename = self
            .config
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| CONFIG_FILE.into());

        if filename.exists() {
            Some(filename)
        } else {
            None
        }
    }

    /// Apply changes to the config after it's been loaded, e.g. overriding
    /// values in a config file using command-line options.
    ///
    /// This can be safely deleted if you don't want to override config
    /// settings from command-line options.
    fn process_config(&self, config: AppConfig) -> Result<AppConfig, FrameworkError> {
        Ok(config)
    }
}
