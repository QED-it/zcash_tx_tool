//! `sync` subcommand - initialize the application, generate keys from seed phrase and sync with the blockchain

use crate::prelude::*;
use crate::config::ZsaWalletConfig;
use abscissa_core::{config, Command, FrameworkError, Runnable};
use zcash_primitives::consensus::BlockHeight;
use crate::components::rpc_client::mock::MockZcashNode;
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
        info!("Seed phrase: {}", &config.wallet.seed_phrase);

        let mut rpc = MockZcashNode::new();
        let mut wallet = Wallet::new();

        sync(&mut wallet, &mut rpc)
    }
}

pub fn sync(wallet: &mut Wallet, rpc: &mut MockZcashNode) {
    info!("Starting sync");

    loop {
        let next_height: BlockHeight = match wallet.last_block_height() {
            Some(height) => height + 1,
            None => BlockHeight::from_u32(0)
        };

        let block = match rpc.get_block(next_height.into()) {
            Ok(block) => block,
            Err(err) => {
                info!("No block at height {}: {}", next_height, err);
                return
            }
        };

        if true /* block.prev_hash == wallet.last_block_hash */ {
            info!("Adding transactions from block {} at height {}", block.hash, block.height);
            let transactions = block.tx_ids.into_iter().map(| tx_id| rpc.get_transaction(tx_id).unwrap()).collect();
            wallet.add_notes_from_block(block.height, block.hash, transactions).unwrap();
        } else {
            // TODO We have a reorg, we need to drop the block and all the blocks after it
            warn!("REORG: dropping block {} at height {}", wallet.last_block_hash().unwrap(), next_height);
            wallet.reorg(next_height - 1);
        }
    }
}