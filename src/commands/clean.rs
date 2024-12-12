//! `clean` - resets DB state

use abscissa_core::{Command, Runnable};

use crate::components::user::User;
use crate::prelude::*;

/// Clean state
#[derive(clap::Parser, Command, Debug)]
pub struct CleanCmd {}

impl Runnable for CleanCmd {
    /// Run the `clean` subcommand.
    fn run(&self) {
        let config = APP.config();
        let mut wallet = User::new(&config.wallet.seed_phrase, &config.wallet.miner_seed_phrase);

        wallet.reset();
    }
}
