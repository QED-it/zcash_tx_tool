//! SQLite-backed block data storage for tracking previously scanned blocks.
//!
//! This module provides persistent block data storage in the same SQLite
//! database as the wallet state (via `DATABASE_URL`), enabling:
//! - Resumable sync from the last stored block
//! - Chain reorganization detection by verifying block hash continuity

use diesel::prelude::*;
use diesel::sql_query;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::{env, fmt};

const CREATE_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS block_data (
    height INTEGER PRIMARY KEY NOT NULL,
    hash TEXT NOT NULL,
    prev_hash TEXT NOT NULL
);
"#;

/// A single stored block entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub hash: String,
    pub prev_hash: String,
}

#[derive(Debug)]
pub struct BlockData {
    /// Map of block height to block info
    pub(crate) blocks: BTreeMap<u32, BlockInfo>,
    /// True when in-memory state has changed and should be persisted.
    unsaved: bool,
}

impl BlockData {
    /// Load the block data from SQLite, or create a new empty storage.
    pub fn load() -> Self {
        let database_url = database_url();
        Self::load_from_url(&database_url)
    }

    fn load_from_url(database_url: &str) -> Self {
        let mut conn = establish_connection_with_url(database_url);
        ensure_table(&mut conn);

        let mut block_data = Self {
            blocks: BTreeMap::new(),
            unsaved: false,
        };

        use crate::schema::block_data::dsl as bd;
        let rows: Vec<BlockDataRow> = bd::block_data
            .select(BlockDataRow::as_select())
            .order(bd::height.asc())
            .load(&mut conn)
            .unwrap_or_default();

        for row in rows {
            let height_u32 = match u32::try_from(row.height) {
                Ok(h) => h,
                Err(_) => continue,
            };
            block_data.blocks.insert(
                height_u32,
                BlockInfo {
                    hash: row.hash,
                    prev_hash: row.prev_hash,
                },
            );
        }

        block_data
    }

    /// Save the block data to SQLite.
    pub fn save(&mut self) {
        if !self.unsaved {
            return;
        }

        let database_url = database_url();
        let mut conn = establish_connection_with_url(&database_url);
        self.save_to_connection(&mut conn);
    }

    fn save_to_connection(&mut self, conn: &mut SqliteConnection) {
        ensure_table(conn);

        let new_rows = self
            .blocks
            .iter()
            .filter_map(|(h, b)| {
                let height_i32 = i32::try_from(*h).ok()?;
                Some(NewBlockDataRow {
                    height: height_i32,
                    hash: b.hash.clone(),
                    prev_hash: b.prev_hash.clone(),
                })
            })
            .collect::<Vec<_>>();

        let res = conn.transaction(|conn| {
            use crate::schema::block_data::dsl as bd;
            diesel::delete(bd::block_data).execute(conn)?;
            if !new_rows.is_empty() {
                diesel::insert_into(bd::block_data)
                    .values(&new_rows)
                    .execute(conn)?;
            }
            Ok::<_, diesel::result::Error>(())
        });

        if res.is_ok() {
            self.unsaved = false;
        }
    }

    /// Get the last (highest) stored block height
    pub fn last_height(&self) -> Option<u32> {
        self.blocks.keys().last().copied()
    }

    /// Get a stored block by height
    pub fn get(&self, height: u32) -> Option<&BlockInfo> {
        self.blocks.get(&height)
    }

    /// Insert a block into the storage
    pub fn insert(&mut self, height: u32, hash: String, prev_hash: String) {
        self.blocks.insert(
            height,
            BlockInfo {
                hash,
                prev_hash,
            },
        );
        self.unsaved = true;
    }

    /// Remove all blocks from the given height onwards (for reorg handling)
    pub fn truncate_from(&mut self, from_height: u32) {
        self.blocks.retain(|&h, _| h < from_height);
        self.unsaved = true;
    }

    /// Clear all stored blocks
    pub fn clear(&mut self) {
        self.blocks.clear();
        self.unsaved = true;
    }

    /// Clear the persistent block data from SQLite.
    pub fn clear_from_db() {
        let database_url = database_url();
        Self::delete_from_url(&database_url);
    }

    fn delete_from_url(database_url: &str) {
        let mut conn = establish_connection_with_url(database_url);
        ensure_table(&mut conn);
        use crate::schema::block_data::dsl as bd;
        let _ = diesel::delete(bd::block_data).execute(&mut conn);
    }
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::block_data)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct BlockDataRow {
    height: i32,
    hash: String,
    prev_hash: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::block_data)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct NewBlockDataRow {
    height: i32,
    hash: String,
    prev_hash: String,
}

fn ensure_table(conn: &mut SqliteConnection) {
    let _ = sql_query(CREATE_TABLE_SQL).execute(conn);
}

fn establish_connection_with_url(database_url: &str) -> SqliteConnection {
    SqliteConnection::establish(database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", Redacted(database_url)))
}

/// Avoid printing full database URLs/paths in panics.
struct Redacted<'a>(&'a str);
impl fmt::Display for Redacted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = self.0;
        write!(f, "<redacted DATABASE_URL>")
    }
}

fn database_url() -> String {
    dotenvy::dotenv().ok();
    env::var("DATABASE_URL").expect("DATABASE_URL must be set")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_block_data_operations() {
        let db = NamedTempFile::new().unwrap();
        let database_url = db.path().to_string_lossy().to_string();

        BlockData::delete_from_url(&database_url);

        let data = BlockData::load_from_url(&database_url);
        assert!(data.last_height().is_none());

        let mut data = BlockData::load_from_url(&database_url);
        data.insert(100, "hash100".to_string(), "prev100".to_string());
        data.insert(101, "hash101".to_string(), "hash100".to_string());
        data.insert(102, "hash102".to_string(), "hash101".to_string());

        assert_eq!(data.last_height(), Some(102));

        let block = data.get(101).unwrap();
        assert_eq!(block.hash, "hash101");
        assert_eq!(block.prev_hash, "hash100");

        let mut conn = establish_connection_with_url(&database_url);
        data.save_to_connection(&mut conn);
        let reloaded = BlockData::load_from_url(&database_url);
        assert_eq!(reloaded.last_height(), Some(102));
        let block = reloaded.get(101).unwrap();
        assert_eq!(block.hash, "hash101");

        let mut data = reloaded;
        data.truncate_from(101);
        assert_eq!(data.last_height(), Some(100));
        assert!(data.get(101).is_none());
        assert!(data.get(102).is_none());

        data.clear();
        assert!(data.last_height().is_none());

        BlockData::delete_from_url(&database_url);
    }

    #[test]
    fn test_partial_block_data_invalidation() {
        let db = NamedTempFile::new().unwrap();
        let database_url = db.path().to_string_lossy().to_string();

        let mut data = BlockData::load_from_url(&database_url);

        for i in 100..=110 {
            let hash = format!("hash{}", i);
            let prev_hash = if i == 100 {
                "genesis".to_string()
            } else {
                format!("hash{}", i - 1)
            };
            data.insert(i, hash, prev_hash);
        }

        assert_eq!(data.last_height(), Some(110));

        for i in 100..=110 {
            assert!(data.get(i).is_some(), "Block {} should exist", i);
        }

        data.truncate_from(106);

        for i in 100..=105 {
            assert!(
                data.get(i).is_some(),
                "Block {} should still exist after truncate",
                i
            );
        }

        for i in 106..=110 {
            assert!(
                data.get(i).is_none(),
                "Block {} should be removed after truncate",
                i
            );
        }

        assert_eq!(data.last_height(), Some(105));

        for i in 101..=105 {
            let block = data.get(i).unwrap();
            let expected_prev = format!("hash{}", i - 1);
            assert_eq!(
                block.prev_hash, expected_prev,
                "Block {} prev_hash mismatch",
                i
            );
        }
    }
}
