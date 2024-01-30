use crate::prelude::*;
use abscissa_core::{Command, Runnable};
use zcash_client_backend::encoding::AddressCodec;
use zcash_primitives::consensus::TEST_NETWORK;
use zcash_primitives::zip339::{Count, Mnemonic};
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::wallet::Wallet;

#[derive(clap::Parser, Command, Debug)]
pub struct TestConfig {
    seed_phrase: Vec<String>,
}


impl Runnable for TestConfig {
    /// Run the `sync` subcommand.
    fn run(&self) {
        let config = APP.config();

        info!("Seed phrase: {}", config.wallet.seed_phrase);

        let mut wallet = Wallet::new(&config.wallet.seed_phrase);

        let taddr = wallet.miner_address();

        let staddr: String = taddr.encode(&TEST_NETWORK);

        info!("T-Addr: {}", staddr);
    }
}
