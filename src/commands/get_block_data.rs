//! Subcommand to query block hashes from the local db.
//!
//! During sync the wallet records the hash of every block it processes.
//! This command prints the hash (and the previous block's hash) for a
//! given height, or for the most recently synced block when no height is
//! specified.
//!
//! Usage:
//!   tx_tool get-block-data          # last synced block
//!   tx_tool get-block-data <HEIGHT> # specific block

use abscissa_core::{Command, Runnable};
use serde::Serialize;

use crate::components::block_data::BlockData;

#[derive(Serialize)]
struct BlockDataResult {
    block_height: Option<u32>,
    hash: Option<String>,
    prev_hash: Option<String>,
    error: Option<String>,
}

/// Query block hash data from the local db.
#[derive(clap::Parser, Command, Debug)]
pub struct GetBlockDataCmd {
    /// Block height to retrieve (default: last block).
    pub block_height: Option<u32>,
}

impl Runnable for GetBlockDataCmd {
    fn run(&self) {
        let mut block_data = BlockData::new();

        let height = self.block_height.or_else(|| block_data.last_height_block());

        let result = match height {
            Some(height) => match block_data.get_hash(height) {
                Some(hash) => {
                    let prev_hash = height
                        .checked_sub(1)
                        .and_then(|h| block_data.get_hash(h));
                    BlockDataResult {
                        block_height: Some(height),
                        hash: Some(hash),
                        prev_hash,
                        error: None,
                    }
                }
                None => BlockDataResult {
                    block_height: Some(height),
                    hash: None,
                    prev_hash: None,
                    error: Some(format!("Block at height {} not found", height)),
                },
            },
            None => BlockDataResult {
                block_height: None,
                hash: None,
                prev_hash: None,
                error: Some("No blocks found in local db".to_string()),
            },
        };

        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    }
}
