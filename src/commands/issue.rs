//! `issue` - issue assets

use abscissa_core::{Command, Runnable};
use crate::prelude::*;
use crate::components::rpc_client::RpcClient;
use crate::components::wallet::Wallet;


/// `issue` subcommand
#[derive(clap::Parser, Command, Debug)]
pub struct IssueCmd {
}

impl Runnable for IssueCmd {
    /// Run the `issue` subcommand.
    fn run(&self) {
        let config = APP.config();

        let rpc_client = RpcClient::new();
        let wallet = Wallet::new();

        todo!()
    }
}