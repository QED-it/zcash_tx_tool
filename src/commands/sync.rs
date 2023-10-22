//! `sync` subcommand - initialize the application, generate keys from seed phrase and sync with the blockchain

use crate::prelude::*;
use crate::config::ZsaWalletConfig;
use abscissa_core::{config, Command, FrameworkError, Runnable};
use zcash_primitives::consensus::BlockHeight;
use crate::components::rpc_client::reqwest::ReqwestRpcClient;
use crate::components::rpc_client::RpcClient;
use crate::components::wallet::Wallet;
use crate::model::Block;

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
        info!("Seed phrase: {}", &config.wallet.seed_phrase);

        let rpc = ReqwestRpcClient::new();
        let mut wallet = Wallet::empty();

        info!("Starting sync");

        loop {
            let best_block_hash = match rpc.get_best_block_hash() {
                Ok(hash) => hash,
                Err(error) => panic!("Error getting best block hash: {}", error)
                // TODO handle empty blockchain?
            };

            info!("Best block hash: {}", best_block_hash);

            if let Some(wallet_last_hash) = wallet.last_block_hash()  {
                if best_block_hash == wallet_last_hash {
                    // We are in a fully synced data, sync is successfully over
                    return
                }
            } else {
                // We are not in a synced state, we either need to get new blocks or do a reorg
                let height: BlockHeight = match wallet.last_block_height() {
                    Some(height) => height + 1,
                    None => BlockHeight::from_u32(0),
                };

                let block: Block = rpc.get_block(height.into()).unwrap();

                if height == BlockHeight::from_u32(0) {
                    // We are dealing with genesis block
                    let transactions = block.tx_ids.into_iter().map(| tx_id| rpc.get_transaction(tx_id).unwrap()).collect();
                    wallet.add_notes_from_block(block.height, block.hash, transactions).unwrap();
                } else {
                    let wallet_last_hash = wallet.last_block_hash().unwrap(); // it's ok to panic when we don't have block at height != 0
                    if wallet_last_hash == block.previous_block_hash {
                        // We are in a normal state, just add the block to the cache and wallet
                        let transactions = block.tx_ids.into_iter().map(| tx_id| rpc.get_transaction(tx_id).unwrap()).collect();
                        wallet.add_notes_from_block(block.height, block.hash, transactions).unwrap();
                    } else {
                        // We are in a reorg state, we need to drop the block and all the blocks after it
                        warn!("REORG: dropping block {} at height {}", wallet_last_hash, height);
                        wallet.reorg(height - 1);
                    }
                }
            }
        }
    }
}
