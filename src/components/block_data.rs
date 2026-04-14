//! SQLite-backed local db for block hashes, used for resumable sync.

use crate::components::db;
use diesel::dsl::max;
use diesel::prelude::*;
use diesel::sql_query;

const CREATE_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS block_data (
    height INTEGER PRIMARY KEY NOT NULL,
    hash TEXT NOT NULL
);
"#;

pub struct BlockData {
    conn: SqliteConnection,
}

impl BlockData {
    pub fn new() -> Self {
        Self::new_with_url(&db::database_url())
    }

    fn new_with_url(database_url: &str) -> Self {
        let mut conn = db::establish_connection(database_url);
        let _ = sql_query(CREATE_TABLE_SQL).execute(&mut conn);
        Self { conn }
    }

    /// Get the hash of the block at the given height.
    pub fn get_hash(&mut self, height: u32) -> Option<String> {
        use crate::schema::block_data::dsl as bd;
        let height_i32 = i32::try_from(height).ok()?;
        bd::block_data
            .filter(bd::height.eq(height_i32))
            .select(bd::hash)
            .first::<String>(&mut self.conn)
            .optional()
            .expect("Error querying block data")
    }

    /// Insert a block hash into the local db.
    pub fn insert(&mut self, height: u32, hash: String) {
        use crate::schema::block_data::dsl as bd;
        let height_i32 = i32::try_from(height).expect("height too large");
        diesel::insert_into(bd::block_data)
            .values((bd::height.eq(height_i32), bd::hash.eq(hash)))
            .execute(&mut self.conn)
            .expect("Error inserting block data");
    }

    /// Get the last (highest) stored block height.
    pub fn last_height_block(&mut self) -> Option<u32> {
        use crate::schema::block_data::dsl as bd;
        bd::block_data
            .select(max(bd::height))
            .first::<Option<i32>>(&mut self.conn)
            .expect("Error querying max block height")
            .and_then(|h| u32::try_from(h).ok())
    }

    /// Remove all blocks from the given height onwards.
    pub fn truncate_from(&mut self, from_height: u32) {
        use crate::schema::block_data::dsl as bd;
        let height_i32 = i32::try_from(from_height).expect("height too large");
        diesel::delete(bd::block_data.filter(bd::height.ge(height_i32)))
            .execute(&mut self.conn)
            .expect("Error truncating block data");
    }

    /// Clear all stored blocks.
    pub fn clear(&mut self) {
        use crate::schema::block_data::dsl as bd;
        diesel::delete(bd::block_data)
            .execute(&mut self.conn)
            .expect("Error clearing block data");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_block_data_operations() {
        let db = NamedTempFile::new().unwrap();
        let database_url = db.path().to_string_lossy().to_string();

        let mut data = BlockData::new_with_url(&database_url);
        assert!(data.last_height_block().is_none());

        data.insert(100, "hash100".to_string());
        data.insert(101, "hash101".to_string());
        data.insert(102, "hash102".to_string());

        assert_eq!(data.last_height_block(), Some(102));
        assert_eq!(data.get_hash(101).as_deref(), Some("hash101"));

        // Verify persistence by reading from a new connection
        let mut reloaded = BlockData::new_with_url(&database_url);
        assert_eq!(reloaded.last_height_block(), Some(102));
        assert_eq!(reloaded.get_hash(101).as_deref(), Some("hash101"));

        data.truncate_from(101);
        assert_eq!(data.last_height_block(), Some(100));
        assert!(data.get_hash(101).is_none());
        assert!(data.get_hash(102).is_none());

        data.clear();
        assert!(data.last_height_block().is_none());
    }

    #[test]
    fn test_block_data_truncation() {
        let db = NamedTempFile::new().unwrap();
        let database_url = db.path().to_string_lossy().to_string();

        let mut data = BlockData::new_with_url(&database_url);

        for i in 100..=110 {
            data.insert(i, format!("hash{}", i));
        }

        assert_eq!(data.last_height_block(), Some(110));

        data.truncate_from(106);

        for i in 100..=105 {
            assert!(data.get_hash(i).is_some(), "Block {} should still exist", i);
        }

        for i in 106..=110 {
            assert!(data.get_hash(i).is_none(), "Block {} should be removed", i);
        }

        assert_eq!(data.last_height_block(), Some(105));
    }
}
