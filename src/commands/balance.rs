//! `getbalance` - get wallet balance per asset

use abscissa_core::{Command, Runnable};
use crate::prelude::*;
use crate::components::wallet::Wallet;


/// `getbalance` subcommand
#[derive(clap::Parser, Command, Debug)]
pub struct GetBalanceCmd {
}

impl Runnable for GetBalanceCmd {
    /// Run the `getbalance` subcommand.
    fn run(&self) {
        let config = APP.config();

        let wallet = Wallet::new();

        todo!()
    }
}