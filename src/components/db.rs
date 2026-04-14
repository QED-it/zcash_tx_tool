//! Centralised database connection helpers.
//!
//! Every module that needs SQLite access should go through these functions
//! so the default URL and env-var override live in exactly one place.

use diesel::prelude::*;
use std::env;

const DEFAULT_DATABASE_URL: &str = "walletdb.sqlite";

/// Return the database URL from `DATABASE_URL` env var, falling back to the
/// default `walletdb.sqlite`.
pub fn database_url() -> String {
    env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string())
}

/// Like [`database_url`] but returns `None` when `DATABASE_URL` is unset *and*
/// the default file does not exist on disk. Useful for code paths that must not
/// create a DB as a side-effect (e.g. load/delete during init).
pub fn try_database_url() -> Option<String> {
    match env::var("DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) => {
            if std::path::Path::new(DEFAULT_DATABASE_URL).exists() {
                Some(DEFAULT_DATABASE_URL.to_string())
            } else {
                None
            }
        }
    }
}

/// Open a SQLite connection to the given URL.
pub fn establish_connection(database_url: &str) -> SqliteConnection {
    SqliteConnection::establish(database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
