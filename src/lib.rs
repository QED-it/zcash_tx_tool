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

/// Print an informational message to stdout for human-readable commands.
/// Suppressed entirely for machine-readable subcommands (e.g. `get-block-data`)
/// that require clean, noise-free stdout.
pub fn print_info(msg: &str) {
    if !std::env::args().any(|a| a == "get-block-data") {
        println!("{}", msg);
    }
}
