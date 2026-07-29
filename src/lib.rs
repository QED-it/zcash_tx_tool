//! Zcash Tx Tool
//!
//! Application based on the [Abscissa] framework.
//!
//! [Abscissa]: https://github.com/iqlusioninc/abscissa

// Tip: Deny warnings with `RUSTFLAGS="-D warnings"` environment variable in CI

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, trivial_casts, unused_lifetimes)]

pub mod application;
pub mod commands;
pub mod components;
pub mod config;
pub mod error;
mod model;
pub mod prelude;
mod schema;

/// Subcommands that produce clean, user-facing stdout (the wallet commands
/// and machine-readable outputs); startup banner/config prints are suppressed
/// for them and their default tracing level is reduced to warnings.
pub const QUIET_COMMANDS: &[&str] = &[
    "get-block-data",
    "status",
    "sync",
    "addresses",
    "balance",
    "notes",
    "assets",
    "issue",
    "transfer",
    "burn",
    "finalize",
    "mine",
    "shield",
    "clean",
];

/// Whether the current invocation runs one of the [`QUIET_COMMANDS`].
pub fn is_quiet_invocation() -> bool {
    std::env::args().any(|a| QUIET_COMMANDS.contains(&a.as_str()))
}

/// Print an informational message to stdout for human-readable commands.
/// Suppressed entirely for subcommands that require clean stdout
/// (wallet commands and `get-block-data`).
pub fn print_info(msg: &str) {
    if !is_quiet_invocation() {
        println!("{}", msg);
    }
}
