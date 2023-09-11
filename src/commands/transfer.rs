//! `transfer` - transfer assets

use abscissa_core::{Command, Runnable};
use crate::prelude::*;
use crate::components::rpc_client::RpcClient;
use crate::components::wallet::Wallet;


/// `transfer` subcommand
#[derive(clap::Parser, Command, Debug)]
pub struct TransferCmd {
}

impl Runnable for TransferCmd {
    /// Run the `transfer` subcommand.
    fn run(&self) {
        let config = APP.config();

        let rpc_client = RpcClient::new();
        let wallet = Wallet::new();

        todo!()
    }
}