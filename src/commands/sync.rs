//! `sync` subcommand - initialize the application, generate keys from seed phrase and sync with the blockchain

use std::time::Duration;
use std::thread::sleep;

use crate::prelude::*;
use crate::config::ZsaWalletConfig;
use abscissa_core::{config, Command, FrameworkError, Runnable};
use crate::components::block_cache::BlockCache;
use crate::components::rpc_client::RpcClient;
use crate::components::wallet::Wallet;

/// `sync` subcommand
#[derive(clap::Parser, Command, Debug)]
pub struct SyncCmd {
    /// Seed phrase to generate keys from, separated by spaces. If not provided, a random seed phrase will be generated
    seed_phrase: Vec<String>,
}

impl config::Override<ZsaWalletConfig> for SyncCmd {
    // Process the given command line options, overriding settings from
    // a configuration file using explicit flags taken from command-line
    // arguments.
    fn override_config(
        &self,
        mut config: ZsaWalletConfig,
    ) -> Result<ZsaWalletConfig, FrameworkError> {
        if self.seed_phrase.is_empty() {
            // Generate a random seed phrase
            // TODO make it as bit more random
            config.wallet.seed_phrase = "zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra".to_string();
        } else {
            // TODO check if the seed phrase is valid
        }

        Ok(config)
    }
}


impl Runnable for SyncCmd {
    /// Run the `sync` subcommand.
    fn run(&self) {
        let config = APP.config();
        println!("Seed phrase: {}", &config.wallet.seed_phrase);

        let rpc = RpcClient::new();
        let mut blocks = BlockCache::new();
        let wallet = Wallet::new();

        info!("Starting sync");

        while !self.interrupted {
            let best_block_hash = rpc.get_best_block_hash();
            if best_block_hash == blocks.latest_hash {
                // We are in a fully synced data, sync is successfully over
                sleep(Duration::from_secs(2));
                return
            } else {
                // We are not in a synced state, we either need to get new blocks or do a reorg
                let height = blocks.latest_height + 1;
                let block = rpc.get_block(height);

                // TODO handle no block at this height

                if blocks.latest_hash == block.prev_hash {
                    // We are in a normal state, just add the block to the cache and wallet
                    blocks.add(&block);
                    wallet.add_notes_from_block(&block);
                } else {
                    // We are in a reorg state, we need to drop the block and all the blocks after it
                    warn!("REORG: dropping block {} at height {}", blocks.latest_hash, height);
                    blocks.reorg(height - 1);
                }
            }
        }
    }
}
