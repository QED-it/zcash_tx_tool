//! `mine` — produce blocks on the regtest node.
//!
//! On regtest, block production is driven by the wallet itself through the
//! node's `getblocktemplate`/`submitblock` RPCs. Any transactions waiting in
//! the node mempool are included in the produced blocks.

use abscissa_core::{Command, Runnable};

use crate::commands::wallet::{exit_err, WalletCtx};
use crate::components::transactions::mine_empty_blocks;

/// Mine block(s) on the regtest node (includes mempool transactions)
#[derive(clap::Parser, Command, Debug)]
pub struct MineCmd {
    /// Number of blocks to mine
    #[arg(long, default_value_t = 1)]
    pub blocks: u32,
}

impl Runnable for MineCmd {
    fn run(&self) {
        let mut ctx = WalletCtx::load();
        ctx.require_node();
        ctx.sync();

        if self.blocks == 0 {
            exit_err("--blocks must be at least 1");
        }

        match mine_empty_blocks(self.blocks, &mut ctx.rpc) {
            Ok((first_height, _)) => {
                println!(
                    "✔ mined {} block(s), heights {}..{}",
                    self.blocks,
                    first_height,
                    first_height + self.blocks - 1
                );
            }
            Err(e) => exit_err(&format!("mining failed: {}", e)),
        }
        ctx.sync();
    }
}
