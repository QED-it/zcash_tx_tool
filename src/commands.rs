//! ZsaWallet Subcommands
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

mod sync;
mod issue;
mod transfer;
mod burn;
mod balance;
mod test;

use self::sync::SyncCmd;
use self::transfer::TransferCmd;
use crate::config::ZsaWalletConfig;
use abscissa_core::{config::Override, Command, Configurable, FrameworkError, Runnable};
use std::path::PathBuf;
use crate::commands::balance::GetWalletInfoCmd;
use crate::commands::burn::BurnCmd;
use crate::commands::issue::IssueCmd;
use crate::commands::test::TestCmd;

/// ZsaWallet Configuration Filename
pub const CONFIG_FILE: &str = "zsa_wallet.toml";

/// ZsaWallet Subcommands
/// Subcommands need to be listed in an enum.
#[derive(clap::Parser, Command, Debug, Runnable)]
pub enum ZsaWalletCmd {
    /// Initialize the application, generate keys from seed phrase and sync with the blockchain
    Sync(SyncCmd), Transfer(TransferCmd), Issue(IssueCmd), Burn(BurnCmd), Balance(GetWalletInfoCmd), Test(TestCmd)
}

/// Entry point for the application. It needs to be a struct to allow using subcommands!
#[derive(clap::Parser, Command, Debug)]
#[command(author, about, version)]
pub struct EntryPoint {
    #[command(subcommand)]
    cmd: ZsaWalletCmd,

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
impl Configurable<ZsaWalletConfig> for EntryPoint {
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
    fn process_config(
        &self,
        config: ZsaWalletConfig,
    ) -> Result<ZsaWalletConfig, FrameworkError> {
        match &self.cmd {
            ZsaWalletCmd::Sync(cmd) => cmd.override_config(config),
            // If you don't need special overrides for some
            // subcommands, you can just use a catch all
            _ => Ok(config),
        }
    }
}
