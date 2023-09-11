//! ZsaWallet
//!
//! Application based on the [Abscissa] framework.
//!
//! [Abscissa]: https://github.com/iqlusioninc/abscissa

// Tip: Deny warnings with `RUSTFLAGS="-D warnings"` environment variable in CI

#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    trivial_casts,
    unused_lifetimes,
    unused_qualifications
)]

use std::env;
use diesel::{Connection, SqliteConnection};
use dotenvy::dotenv;

pub mod application;
pub mod config;
pub mod error;
pub mod prelude;
pub mod commands;
pub mod components;
mod schema;

