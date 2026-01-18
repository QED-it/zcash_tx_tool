//! SQLite-backed block cache for tracking previously scanned blocks.
//!
//! This module provides a lightweight caching mechanism stored in the same SQLite
//! database as the wallet state (via `DATABASE_URL`), enabling:
//! - Resumable sync from the last cached block
//! - Chain reorganization detection by verifying block hash continuity
//!
//! Backwards compatibility: if a legacy `block_cache.json` file is present, it will be
//! imported into SQLite on load and then removed.

use diesel::prelude::*;
use diesel::sql_query;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::{env, fmt};

const CACHE_FILE_NAME: &str = "block_cache.json";
const CREATE_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS block_cache (
    height INTEGER PRIMARY KEY NOT NULL,
    hash TEXT NOT NULL,
    prev_hash TEXT NOT NULL,
    tx_hex_json TEXT NOT NULL DEFAULT '[]'
);
"#;

/// A single cached block entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedBlock {
    pub hash: String,
    pub prev_hash: String,
    /// Hex-encoded full transactions for this block.
    /// Optional for backwards compatibility with older cache files.
    #[serde(default)]
    pub tx_hex: Vec<String>,
}

#[derive(Debug)]
pub struct BlockCache {
    /// Map of block height to block data
    pub(crate) blocks: BTreeMap<u32, CachedBlock>,
    dirty: bool,
}

impl BlockCache {
    /// Load the block cache from SQLite, or create a new empty cache.
    pub fn load() -> Self {
        let database_url = database_url();
        Self::load_from_url(&database_url)
    }

    fn load_from_url(database_url: &str) -> Self {
        let mut conn = establish_connection_with_url(database_url);
        ensure_table(&mut conn);

        let mut cache = Self {
            blocks: BTreeMap::new(),
            dirty: false,
        };

        // If a legacy JSON cache exists, import it first (best-effort).
        cache.import_legacy_json_if_present(&mut conn);

        // Load from SQLite.
        use crate::schema::block_cache::dsl as bc;
        let rows: Vec<BlockCacheRow> = bc::block_cache
            .select(BlockCacheRow::as_select())
            .order(bc::height.asc())
            .load(&mut conn)
            .unwrap_or_default();

        for row in rows {
            let height_u32 = match u32::try_from(row.height) {
                Ok(h) => h,
                Err(_) => continue,
            };
            let tx_hex: Vec<String> = serde_json::from_str(&row.tx_hex_json).unwrap_or_default();
            cache.blocks.insert(
                height_u32,
                CachedBlock {
                    hash: row.hash,
                    prev_hash: row.prev_hash,
                    tx_hex,
                },
            );
        }

        cache
    }

    /// Save the block cache to SQLite.
    pub fn save(&mut self) {
        if !self.dirty {
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
                let tx_hex_json = serde_json::to_string(&b.tx_hex).ok()?;
                Some(NewBlockCacheRow {
                    height: height_i32,
                    hash: b.hash.clone(),
                    prev_hash: b.prev_hash.clone(),
                    tx_hex_json,
                })
            })
            .collect::<Vec<_>>();

        let res = conn.transaction(|conn| {
            use crate::schema::block_cache::dsl as bc;
            diesel::delete(bc::block_cache).execute(conn)?;
            if !new_rows.is_empty() {
                diesel::insert_into(bc::block_cache)
                    .values(&new_rows)
                    .execute(conn)?;
            }
            Ok::<_, diesel::result::Error>(())
        });

        if res.is_ok() {
            self.dirty = false;
        }
    }

    /// Get the path to the cache file
    fn cache_path() -> PathBuf {
        // Keep the cache stable regardless of the current working directory.
        // Without anchoring the path, running the binary from a different CWD
        // would create a new cache file and force a full re-download of blocks.
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(CACHE_FILE_NAME)
    }

    /// Get the last (highest) cached block height
    pub fn last_height(&self) -> Option<u32> {
        self.blocks.keys().last().copied()
    }

    /// Get a cached block by height
    pub fn get(&self, height: u32) -> Option<&CachedBlock> {
        self.blocks.get(&height)
    }

    /// Insert a block into the cache
    pub fn insert(&mut self, height: u32, hash: String, prev_hash: String, tx_hex: Vec<String>) {
        self.blocks.insert(
            height,
            CachedBlock {
                hash,
                prev_hash,
                tx_hex,
            },
        );
        self.dirty = true;
    }

    /// Iterate cached blocks in height order.
    pub fn blocks_iter(&self) -> impl Iterator<Item = (&u32, &CachedBlock)> {
        self.blocks.iter()
    }

    /// Returns true if (and only if) every cached block has tx data.
    pub fn has_complete_tx_data(&self) -> bool {
        self.blocks.values().all(|b| !b.tx_hex.is_empty())
    }

    /// Remove all blocks from the given height onwards (for reorg handling)
    pub fn truncate_from(&mut self, from_height: u32) {
        self.blocks.retain(|&h, _| h < from_height);
        self.dirty = true;
    }

    /// Clear the entire cache
    pub fn clear(&mut self) {
        self.blocks.clear();
        self.dirty = true;
    }

    /// Clear the persistent cache from SQLite (and remove any legacy JSON cache file).
    pub fn delete_file() {
        // Remove legacy JSON cache if present.
        let path = Self::cache_path();
        let _ = fs::remove_file(path);

        let database_url = database_url();
        Self::delete_from_url(&database_url);
    }

    fn delete_from_url(database_url: &str) {
        let mut conn = establish_connection_with_url(database_url);
        ensure_table(&mut conn);
        use crate::schema::block_cache::dsl as bc;
        let _ = diesel::delete(bc::block_cache).execute(&mut conn);
    }

    fn import_legacy_json_if_present(&mut self, conn: &mut SqliteConnection) {
        let path = Self::cache_path();
        if !path.exists() {
            return;
        }

        // Only import if SQLite cache is empty.
        use crate::schema::block_cache::dsl as bc;
        let existing: i64 = bc::block_cache.count().get_result(conn).unwrap_or(0);
        if existing > 0 {
            let _ = fs::remove_file(path);
            return;
        }

        let json = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return,
        };
        let legacy: LegacyJsonBlockCache = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(_) => return,
        };

        self.blocks = legacy.blocks;
        self.dirty = true;
        // Persist immediately so that load reads from SQLite consistently.
        self.save_to_connection(conn);
        let _ = fs::remove_file(path);
    }
}

#[derive(Debug, Deserialize, Default)]
struct LegacyJsonBlockCache {
    blocks: BTreeMap<u32, CachedBlock>,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::block_cache)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct BlockCacheRow {
    height: i32,
    hash: String,
    prev_hash: String,
    tx_hex_json: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::block_cache)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct NewBlockCacheRow {
    height: i32,
    hash: String,
    prev_hash: String,
    tx_hex_json: String,
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
    fn test_block_cache_operations() {
        let db = NamedTempFile::new().unwrap();
        let database_url = db.path().to_string_lossy().to_string();

        // Start with a clean slate
        BlockCache::delete_from_url(&database_url);

        // Test 1: Empty cache
        let cache = BlockCache::load_from_url(&database_url);
        assert!(cache.last_height().is_none());
        println!("✓ Empty cache loads correctly");

        // Test 2: Insert blocks
        let mut cache = BlockCache::load_from_url(&database_url);
        cache.insert(
            100,
            "hash100".to_string(),
            "prev100".to_string(),
            vec!["tx100".to_string()],
        );
        cache.insert(
            101,
            "hash101".to_string(),
            "hash100".to_string(),
            vec!["tx101".to_string()],
        );
        cache.insert(
            102,
            "hash102".to_string(),
            "hash101".to_string(),
            vec!["tx102".to_string()],
        );

        assert_eq!(cache.last_height(), Some(102));
        println!("✓ Inserted 3 blocks, last height is 102");

        // Test 3: Get specific block
        let block = cache.get(101).unwrap();
        assert_eq!(block.hash, "hash101");
        assert_eq!(block.prev_hash, "hash100");
        println!("✓ Block 101 retrieved correctly");

        // Test 4: Save and reload
        let mut conn = establish_connection_with_url(&database_url);
        cache.save_to_connection(&mut conn);
        let reloaded = BlockCache::load_from_url(&database_url);
        assert_eq!(reloaded.last_height(), Some(102));
        let block = reloaded.get(101).unwrap();
        assert_eq!(block.hash, "hash101");
        println!("✓ Cache saved and reloaded correctly");

        // Test 5: Truncate (simulate reorg)
        let mut cache = reloaded;
        cache.truncate_from(101);
        assert_eq!(cache.last_height(), Some(100));
        assert!(cache.get(101).is_none());
        assert!(cache.get(102).is_none());
        println!("✓ Truncate from height 101 works (simulates reorg)");

        // Test 6: Clear
        cache.clear();
        assert!(cache.last_height().is_none());
        println!("✓ Cache cleared");

        // Cleanup
        BlockCache::delete_from_url(&database_url);
        println!("✓ Test cleanup complete");
    }

    #[test]
    fn test_partial_cache_invalidation() {
        let db = NamedTempFile::new().unwrap();
        let database_url = db.path().to_string_lossy().to_string();

        // Simulates a scenario where:
        // - Cache has blocks 100-110
        // - Blockchain diverged at block 106 (blocks 100-105 are still valid)
        // - We should keep blocks 100-105 and resync from 106

        let mut cache = BlockCache::load_from_url(&database_url);

        // Insert 11 blocks (100-110)
        for i in 100..=110 {
            let hash = format!("hash{}", i);
            let prev_hash = if i == 100 {
                "genesis".to_string()
            } else {
                format!("hash{}", i - 1)
            };
            cache.insert(i, hash, prev_hash, vec![format!("tx{}", i)]);
        }

        assert_eq!(cache.last_height(), Some(110));
        println!("✓ Cache has blocks 100-110");

        // Verify all blocks exist
        for i in 100..=110 {
            assert!(cache.get(i).is_some(), "Block {} should exist", i);
        }
        println!("✓ All blocks 100-110 exist in cache");

        // Simulate reorg detection: truncate from block 106
        // This keeps blocks 100-105 (the common ancestor chain)
        cache.truncate_from(106);

        // Verify blocks 100-105 still exist
        for i in 100..=105 {
            assert!(
                cache.get(i).is_some(),
                "Block {} should still exist after truncate",
                i
            );
        }
        println!("✓ Blocks 100-105 preserved after truncate");

        // Verify blocks 106-110 are gone
        for i in 106..=110 {
            assert!(
                cache.get(i).is_none(),
                "Block {} should be removed after truncate",
                i
            );
        }
        println!("✓ Blocks 106-110 removed after truncate");

        // Last height should now be 105
        assert_eq!(cache.last_height(), Some(105));
        println!("✓ Last cached height is now 105");

        // Verify chain integrity of remaining blocks
        for i in 101..=105 {
            let block = cache.get(i).unwrap();
            let expected_prev = format!("hash{}", i - 1);
            assert_eq!(
                block.prev_hash, expected_prev,
                "Block {} prev_hash mismatch",
                i
            );
        }
        println!("✓ Remaining chain integrity verified");

        println!("\n Partial cache invalidation works correctly!");
        println!("   - Detected divergence at block 106");
        println!("   - Kept valid blocks 100-105");
        println!("   - Removed invalid blocks 106-110");
        println!("   - Ready to resync from block 106");
    }
}
