//! `get_block_data` - retrieves block data from storage

use abscissa_core::{Command, Runnable};
use serde_json::json;

use crate::components::persistence::sqlite::SqliteDataStorage;

/// Get block data from storage
#[derive(clap::Parser, Command, Debug)]
pub struct GetBlockDataCmd {
    /// Block height to retrieve. If not provided, returns the last block.
    pub block_height: Option<u32>,
}

impl Runnable for GetBlockDataCmd {
    /// Run the `get_block_data` subcommand.
    fn run(&self) {
        let mut db = SqliteDataStorage::new();

        let result = match self.block_height {
            Some(height) => match db.get_block(height) {
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
            },
            None => match db.last_block_height() {
                Some(height) => {
                    let block_info = db.get_block(height).unwrap();
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
            },
        };

        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    }
}
