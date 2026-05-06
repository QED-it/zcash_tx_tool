//! SQLite-backed local db for block hashes, used for resumable sync.
//!
//! Free functions take `&mut SqliteConnection` so callers can participate in
//! a transaction.

use diesel::dsl::max;
use diesel::prelude::*;
use diesel::upsert::excluded;

/// Get the hash of the block at the given height.
pub fn get_hash(conn: &mut SqliteConnection, height: u32) -> Option<String> {
    use crate::schema::block_data::dsl as bd;
    let height_i32 = i32::try_from(height).ok()?;
    bd::block_data
        .filter(bd::height.eq(height_i32))
        .select(bd::hash)
        .first::<String>(conn)
        .optional()
        .expect("Error querying block data")
}

/// Insert or update a block hash in the local db.
pub fn insert(conn: &mut SqliteConnection, height: u32, hash: String) {
    use crate::schema::block_data::dsl as bd;
    let height_i32 = i32::try_from(height).expect("height too large");
    diesel::insert_into(bd::block_data)
        .values((bd::height.eq(height_i32), bd::hash.eq(hash)))
        .on_conflict(bd::height)
        .do_update()
        .set(bd::hash.eq(excluded(bd::hash)))
        .execute(conn)
        .expect("Error inserting block data");
}

/// Get the last (highest) stored block height.
pub fn last_height(conn: &mut SqliteConnection) -> Option<u32> {
    use crate::schema::block_data::dsl as bd;
    bd::block_data
        .select(max(bd::height))
        .first::<Option<i32>>(conn)
        .expect("Error querying max block height")
        .and_then(|h| u32::try_from(h).ok())
}

/// Clear all stored blocks.
pub fn clear(conn: &mut SqliteConnection) {
    use crate::schema::block_data::dsl as bd;
    diesel::delete(bd::block_data)
        .execute(conn)
        .expect("Error clearing block data");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::db;
    use tempfile::NamedTempFile;

    #[test]
    fn test_block_data_operations() {
        let db_file = NamedTempFile::new().unwrap();
        let url = db_file.path().to_string_lossy().to_string();
        let mut c = db::establish_connection(&url);

        assert!(last_height(&mut c).is_none());

        insert(&mut c, 100, "hash100".to_string());
        insert(&mut c, 101, "hash101".to_string());
        insert(&mut c, 102, "hash102".to_string());

        assert_eq!(last_height(&mut c), Some(102));
        assert_eq!(get_hash(&mut c, 101).as_deref(), Some("hash101"));

        insert(&mut c, 101, "hash101-updated".to_string());
        assert_eq!(get_hash(&mut c, 101).as_deref(), Some("hash101-updated"));

        // Verify persistence by reading from a new connection
        let mut c2 = db::establish_connection(&url);
        assert_eq!(last_height(&mut c2), Some(102));
        assert_eq!(get_hash(&mut c2, 101).as_deref(), Some("hash101-updated"));

        clear(&mut c);
        assert!(last_height(&mut c).is_none());
        assert!(get_hash(&mut c, 100).is_none());
    }
}
