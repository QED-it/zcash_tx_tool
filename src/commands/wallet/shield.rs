//! `shield` — regtest ZEC bootstrap: mine coinbase and shield it to Orchard.
//!
//! Mines `maturity` blocks (coinbase needs 100 confirmations before it can be
//! spent), then moves the matured coinbase reward from the transparent miner
//! address into the wallet's shielded Orchard pool.

use abscissa_core::{Command, Runnable};

use crate::commands::wallet::{exit_err, WalletCtx};
use crate::components::miner::MinerKey;
use crate::components::transactions::{create_shield_coinbase_transaction, mine_empty_blocks};
use crate::prelude::*;

/// Mine a coinbase and shield it into the Orchard pool (regtest ZEC faucet)
#[derive(clap::Parser, Command, Debug)]
pub struct ShieldCmd {
    /// Account that receives the shielded ZEC
    #[arg(long, default_value_t = 0)]
    pub to_account: usize,

    /// Blocks to mine so the coinbase matures (consensus requires 100)
    #[arg(long, default_value_t = 100)]
    pub maturity: u32,
}

impl Runnable for ShieldCmd {
    fn run(&self) {
        if self.maturity == 0 {
            exit_err("--maturity must be at least 1: shielding needs a coinbase to spend");
        }

        let config = APP.config();
        let mut ctx = WalletCtx::load();
        ctx.require_node();
        ctx.sync();

        let recipient = ctx.known_account_address(self.to_account);
        let miner_key = MinerKey::new(&config.wallet.miner_seed_phrase);

        println!(
            "mining {} blocks to mature a coinbase reward…",
            self.maturity
        );
        let (height, coinbase_txid) = match mine_empty_blocks(self.maturity, &mut ctx.rpc) {
            Ok(result) => result,
            Err(e) => exit_err(&format!("mining failed: {}", e)),
        };
        println!(
            "✔ coinbase {} at height {} is mature",
            coinbase_txid, height
        );

        let tx = create_shield_coinbase_transaction(
            recipient,
            coinbase_txid,
            &ctx.rpc,
            &mut ctx.wallet,
            &miner_key,
        );

        println!(
            "shielding 625000000 zatoshis (6.25 ZEC) to account {}",
            self.to_account
        );
        ctx.submit(Vec::from([tx]), false);
        ctx.print_balances("Balances after shielding");
    }
}
