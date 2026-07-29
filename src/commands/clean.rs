//! `clean` - resets DB state

use abscissa_core::{Command, Runnable};

use crate::components::db;
use crate::components::wallet::Wallet;
use crate::prelude::*;

/// Clean state
#[derive(clap::Parser, Command, Debug)]
pub struct CleanCmd {}

impl Runnable for CleanCmd {
    /// Run the `clean` subcommand.
    fn run(&self) {
        let config = APP.config();
        let db_path = config.wallet.db_path.as_deref();
        // Reset the database this wallet's config points at, not the default
        // one: now that wallets can each have their own `db_path`,
        // `--config other.toml clean` must wipe that wallet and nothing else.
        let mut c = db::open_wallet(db_path);
        let mut wallet = Wallet::new(&mut c, &config.wallet.seed_phrase);

        wallet.reset(&mut c);

        println!(
            "✔ reset local wallet state in {}",
            db::wallet_database_url(db_path)
        );
    }
}
