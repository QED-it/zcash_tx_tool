//! `get_block_data` - retrieves block data from storage

use abscissa_core::{Command, Runnable};
use serde_json::json;

use crate::components::block_data::BlockData;

/// Get block data from storage
#[derive(clap::Parser, Command, Debug)]
pub struct GetBlockDataCmd {
    /// Block height to retrieve. If not provided, returns the last block.
    pub block_height: Option<u32>,
}

impl Runnable for GetBlockDataCmd {
    /// Run the `get_block_data` subcommand.
    fn run(&self) {
        let block_data = BlockData::load();

        let result = match self.block_height {
            Some(height) => {
                // Get specific block
                match block_data.get(height) {
                    Some(block_info) => json!({
                        "success": true,
                        "block_height": height,
                        "hash": block_info.hash,
                        "prev_hash": block_info.prev_hash,
                        "tx_hex": block_info.tx_hex,
                    }),
                    None => json!({
                        "success": false,
                        "error": format!("Block at height {} not found", height),
                        "block_height": height,
                    }),
                }
            }
            None => {
                // Get last block
                match block_data.last_height() {
                    Some(height) => {
                        let block_info = block_data.get(height).unwrap();
                        json!({
                            "success": true,
                            "block_height": height,
                            "hash": block_info.hash,
                            "prev_hash": block_info.prev_hash,
                            "tx_hex": block_info.tx_hex,
                        })
                    }
                    None => json!({
                        "success": false,
                        "error": "No blocks found in storage",
                        "block_height": null,
                    }),
                }
            }
        };

        // Print JSON result
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    }
}
