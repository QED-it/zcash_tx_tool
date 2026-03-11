use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;

pub struct SqliteDataStorage {
    pub(crate) connection: SqliteConnection,
}

impl SqliteDataStorage {
    pub fn new() -> Self {
        Self {
            connection: establish_connection(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_url(database_url: &str) -> Self {
        Self {
            connection: SqliteConnection::establish(database_url)
                .unwrap_or_else(|_| panic!("Error connecting to {}", database_url)),
        }
    }
}

impl Default for SqliteDataStorage {
    fn default() -> Self {
        Self::new()
    }
}

fn establish_connection() -> SqliteConnection {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
