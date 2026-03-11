//! Block data operations on `SqliteDataStorage`.
//!
//! Provides persistent block header storage for tracking previously scanned blocks,
//! enabling resumable sync and chain reorganization detection.

use crate::components::persistence::model::{BlockDataRow, BlockInfo, NewBlockDataRow};
use crate::components::persistence::sqlite::SqliteDataStorage;
use diesel::prelude::*;

impl SqliteDataStorage {
    /// Get the last (highest) stored block height.
    pub fn last_block_height(&mut self) -> Option<u32> {
        use crate::schema::block_data::dsl as bd;
        bd::block_data
            .select(diesel::dsl::max(bd::height))
            .first::<Option<i32>>(&mut self.connection)
            .ok()
            .flatten()
            .and_then(|h| u32::try_from(h).ok())
    }

    /// Get a stored block by height.
    pub fn get_block(&mut self, height: u32) -> Option<BlockInfo> {
        use crate::schema::block_data::dsl as bd;
        let height_i32 = i32::try_from(height).ok()?;
        let row: BlockDataRow = bd::block_data
            .filter(bd::height.eq(height_i32))
            .select(BlockDataRow::as_select())
            .first(&mut self.connection)
            .ok()?;
        Some(BlockInfo {
            hash: row.hash,
            prev_hash: row.prev_hash,
        })
    }

    /// Insert (or replace) a block header in storage.
    pub fn insert_block(&mut self, height: u32, hash: String, prev_hash: String) {
        let height_i32 = i32::try_from(height).expect("block height overflow");
        use crate::schema::block_data::dsl as bd;
        diesel::replace_into(bd::block_data)
            .values(NewBlockDataRow {
                height: height_i32,
                hash,
                prev_hash,
            })
            .execute(&mut self.connection)
            .expect("failed to insert block data");
    }

    /// Remove all blocks from the given height onwards (for reorg handling).
    pub fn truncate_blocks_from(&mut self, from_height: u32) {
        use crate::schema::block_data::dsl as bd;
        let height_i32 = i32::try_from(from_height).expect("block height overflow");
        diesel::delete(bd::block_data.filter(bd::height.ge(height_i32)))
            .execute(&mut self.connection)
            .expect("failed to truncate block data");
    }

    /// Clear all stored blocks.
    pub fn clear_block_data(&mut self) {
        use crate::schema::block_data::dsl as bd;
        diesel::delete(bd::block_data)
            .execute(&mut self.connection)
            .expect("failed to clear block data");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::sql_query;
    use tempfile::NamedTempFile;

    fn test_db() -> (NamedTempFile, SqliteDataStorage) {
        let db = NamedTempFile::new().unwrap();
        let url = db.path().to_string_lossy().to_string();
        let mut storage = SqliteDataStorage::with_url(&url);
        sql_query(
            "CREATE TABLE IF NOT EXISTS block_data (
                height INTEGER PRIMARY KEY NOT NULL,
                hash TEXT NOT NULL,
                prev_hash TEXT NOT NULL
            )",
        )
        .execute(&mut storage.connection)
        .expect("failed to create block_data table for test");
        (db, storage)
    }

    #[test]
    fn test_block_data_operations() {
        let (_db, mut data) = test_db();
        assert!(data.last_block_height().is_none());

        data.insert_block(100, "hash100".into(), "prev100".into());
        data.insert_block(101, "hash101".into(), "hash100".into());
        data.insert_block(102, "hash102".into(), "hash101".into());
        assert_eq!(data.last_block_height(), Some(102));

        let block = data.get_block(101).unwrap();
        assert_eq!(block.hash, "hash101");
        assert_eq!(block.prev_hash, "hash100");

        data.truncate_blocks_from(101);
        assert_eq!(data.last_block_height(), Some(100));
        assert!(data.get_block(101).is_none());
        assert!(data.get_block(102).is_none());

        data.clear_block_data();
        assert!(data.last_block_height().is_none());
    }

    #[test]
    fn test_partial_block_data_invalidation() {
        let (_db, mut data) = test_db();

        for i in 100..=110 {
            let hash = format!("hash{}", i);
            let prev_hash = if i == 100 {
                "genesis".to_string()
            } else {
                format!("hash{}", i - 1)
            };
            data.insert_block(i, hash, prev_hash);
        }

        assert_eq!(data.last_block_height(), Some(110));
        for i in 100..=110 {
            assert!(data.get_block(i).is_some(), "Block {} should exist", i);
        }

        data.truncate_blocks_from(106);

        for i in 100..=105 {
            assert!(
                data.get_block(i).is_some(),
                "Block {} should still exist after truncate",
                i
            );
        }
        for i in 106..=110 {
            assert!(
                data.get_block(i).is_none(),
                "Block {} should be removed after truncate",
                i
            );
        }

        assert_eq!(data.last_block_height(), Some(105));

        for i in 101..=105 {
            let block = data.get_block(i).unwrap();
            let expected_prev = format!("hash{}", i - 1);
            assert_eq!(
                block.prev_hash, expected_prev,
                "Block {} prev_hash mismatch",
                i
            );
        }
    }
}
