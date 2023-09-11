//! `burn` - burn assets

use abscissa_core::{Command, Runnable};
use crate::prelude::*;
use crate::components::rpc_client::RpcClient;
use crate::components::wallet::Wallet;


/// `burn` subcommand
#[derive(clap::Parser, Command, Debug)]
pub struct BurnCmd {
}

impl Runnable for BurnCmd {
    /// Run the `burn` subcommand.
    fn run(&self) {
        let config = APP.config();

        let rpc_client = RpcClient::new();
        let wallet = Wallet::new();

        todo!()
    }
}