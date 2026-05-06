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
        let mut c = db::open();
        let mut wallet = Wallet::new(&mut c, &config.wallet.seed_phrase);

        wallet.reset(&mut c);
    }
}
