//! `issue` - issue assets

use abscissa_core::{Command, Runnable};
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::prelude::*;
use crate::components::wallet::Wallet;


/// `issue` subcommand
#[derive(clap::Parser, Command, Debug)]
pub struct IssueCmd {
}

impl Runnable for IssueCmd {
    /// Run the `issue` subcommand.
    fn run(&self) {
        let config = APP.config();

        let rpc_client = ReqwestRpcClient::new();
        let wallet = Wallet::empty();

        todo!()
    }
}