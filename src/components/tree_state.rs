//! SQLite-backed persistence for the note commitment tree and sync position.

use bridgetree::{BridgeTree, Checkpoint, MerkleBridge};
use diesel::prelude::*;
use diesel::sql_query;
use incrementalmerkletree::frontier::NonEmptyFrontier;
use incrementalmerkletree::{Address, Level, Position};
use orchard::tree::MerkleHashOrchard;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::components::user::NOTE_COMMITMENT_TREE_DEPTH;
use crate::prelude::info;

const CREATE_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS wallet_state (
    id INTEGER PRIMARY KEY NOT NULL DEFAULT 1,
    commitment_tree_json TEXT NOT NULL,
    last_block_height INTEGER NOT NULL,
    last_block_hash TEXT NOT NULL
);
"#;

#[derive(Serialize, Deserialize)]
struct SerAddress {
    level: u8,
    index: u64,
}

#[derive(Serialize, Deserialize)]
struct SerFrontier {
    position: u64,
    leaf: [u8; 32],
    ommers: Vec<[u8; 32]>,
}

#[derive(Serialize, Deserialize)]
struct SerBridge {
    prior_position: Option<u64>,
    tracking: Vec<SerAddress>,
    ommers: Vec<(SerAddress, [u8; 32])>,
    frontier: SerFrontier,
}

#[derive(Serialize, Deserialize)]
struct SerCheckpoint {
    id: u32,
    bridges_len: usize,
    marked: Vec<u64>,
    forgotten: Vec<u64>,
}

#[derive(Serialize, Deserialize)]
struct SerTree {
    prior_bridges: Vec<SerBridge>,
    current_bridge: Option<SerBridge>,
    saved: Vec<(u64, usize)>,
    checkpoints: Vec<SerCheckpoint>,
    max_checkpoints: usize,
}

fn serialize_tree(
    tree: &BridgeTree<MerkleHashOrchard, u32, NOTE_COMMITMENT_TREE_DEPTH>,
) -> SerTree {
    SerTree {
        prior_bridges: tree.prior_bridges().iter().map(serialize_bridge).collect(),
        current_bridge: tree.current_bridge().as_ref().map(serialize_bridge),
        saved: tree
            .marked_indices()
            .iter()
            .map(|(pos, idx)| (u64::from(*pos), *idx))
            .collect(),
        checkpoints: tree
            .checkpoints()
            .iter()
            .map(serialize_checkpoint)
            .collect(),
        max_checkpoints: tree.max_checkpoints(),
    }
}

fn serialize_bridge(bridge: &MerkleBridge<MerkleHashOrchard>) -> SerBridge {
    SerBridge {
        prior_position: bridge.prior_position().map(u64::from),
        tracking: bridge
            .tracking()
            .iter()
            .map(|addr| SerAddress {
                level: u8::from(addr.level()),
                index: addr.index(),
            })
            .collect(),
        ommers: bridge
            .ommers()
            .iter()
            .map(|(addr, hash)| {
                (
                    SerAddress {
                        level: u8::from(addr.level()),
                        index: addr.index(),
                    },
                    hash.to_bytes(),
                )
            })
            .collect(),
        frontier: serialize_frontier(bridge.frontier()),
    }
}

fn serialize_frontier(frontier: &NonEmptyFrontier<MerkleHashOrchard>) -> SerFrontier {
    SerFrontier {
        position: u64::from(frontier.position()),
        leaf: frontier.leaf().to_bytes(),
        ommers: frontier.ommers().iter().map(|h| h.to_bytes()).collect(),
    }
}

fn serialize_checkpoint(cp: &Checkpoint<u32>) -> SerCheckpoint {
    SerCheckpoint {
        id: *cp.id(),
        bridges_len: cp.bridges_len(),
        marked: cp.marked().iter().map(|p| u64::from(*p)).collect(),
        forgotten: cp.forgotten().iter().map(|p| u64::from(*p)).collect(),
    }
}

fn deserialize_tree(
    ser: SerTree,
) -> Result<BridgeTree<MerkleHashOrchard, u32, NOTE_COMMITMENT_TREE_DEPTH>, String> {
    let prior_bridges: Vec<MerkleBridge<MerkleHashOrchard>> = ser
        .prior_bridges
        .into_iter()
        .map(deserialize_bridge)
        .collect::<Result<_, _>>()?;

    let current_bridge = ser.current_bridge.map(deserialize_bridge).transpose()?;

    let saved: BTreeMap<Position, usize> = ser
        .saved
        .into_iter()
        .map(|(pos, idx)| (Position::from(pos), idx))
        .collect();

    let checkpoints: VecDeque<Checkpoint<u32>> = ser
        .checkpoints
        .into_iter()
        .map(deserialize_checkpoint)
        .collect();

    BridgeTree::from_parts(
        prior_bridges,
        current_bridge,
        saved,
        checkpoints,
        ser.max_checkpoints,
    )
    .map_err(|e| format!("BridgeTree::from_parts failed: {:?}", e))
}

fn deserialize_bridge(ser: SerBridge) -> Result<MerkleBridge<MerkleHashOrchard>, String> {
    let prior_position = ser.prior_position.map(Position::from);

    let tracking: BTreeSet<Address> = ser
        .tracking
        .into_iter()
        .map(|a| Address::from_parts(Level::from(a.level), a.index))
        .collect();

    let ommers: BTreeMap<Address, MerkleHashOrchard> = ser
        .ommers
        .into_iter()
        .map(|(a, bytes)| {
            let addr = Address::from_parts(Level::from(a.level), a.index);
            let hash = Option::from(MerkleHashOrchard::from_bytes(&bytes))
                .ok_or("invalid ommer hash bytes")?;
            Ok((addr, hash))
        })
        .collect::<Result<_, String>>()?;

    let frontier = deserialize_frontier(ser.frontier)?;

    Ok(MerkleBridge::from_parts(
        prior_position,
        tracking,
        ommers,
        frontier,
    ))
}

fn deserialize_frontier(ser: SerFrontier) -> Result<NonEmptyFrontier<MerkleHashOrchard>, String> {
    let position = Position::from(ser.position);
    let leaf = Option::from(MerkleHashOrchard::from_bytes(&ser.leaf))
        .ok_or("invalid frontier leaf hash")?;
    let ommers: Vec<MerkleHashOrchard> = ser
        .ommers
        .into_iter()
        .map(|bytes| {
            Option::from(MerkleHashOrchard::from_bytes(&bytes)).ok_or("invalid frontier ommer hash")
        })
        .collect::<Result<_, _>>()?;

    NonEmptyFrontier::from_parts(position, leaf, ommers)
        .map_err(|e| format!("NonEmptyFrontier::from_parts failed: {:?}", e))
}

fn deserialize_checkpoint(ser: SerCheckpoint) -> Checkpoint<u32> {
    Checkpoint::from_parts(
        ser.id,
        ser.bridges_len,
        ser.marked.into_iter().map(Position::from).collect(),
        ser.forgotten.into_iter().map(Position::from).collect(),
    )
}

// ---------------------------------------------------------------------------
// SQLite persistence
// ---------------------------------------------------------------------------

pub struct LoadedTreeState {
    pub commitment_tree: BridgeTree<MerkleHashOrchard, u32, NOTE_COMMITMENT_TREE_DEPTH>,
    pub last_block_height: u32,
    pub last_block_hash: String,
}

/// Load the saved wallet tree state from SQLite.
/// Returns `None` if no state is stored, the stored data is corrupt, or the
/// database is not reachable (e.g. `DATABASE_URL` not set).
pub fn load_tree_state() -> Option<LoadedTreeState> {
    let database_url = try_database_url()?;
    let mut conn = establish_connection(&database_url);
    ensure_table(&mut conn);

    let row: Option<WalletStateRow> = {
        use crate::schema::wallet_state::dsl as ws;
        ws::wallet_state
            .select(WalletStateRow::as_select())
            .load(&mut conn)
            .unwrap_or_default()
            .into_iter()
            .next()
    };

    let ser_tree: SerTree = serde_json::from_str(&r.commitment_tree_json)
        .map_err(|e| info!("Failed to deserialize saved tree state: {e}"))
        .ok()?;
    let tree = deserialize_tree(ser_tree)
        .map_err(|e| info!("Failed to reconstruct tree: {e}"))
        .ok()?;
    Some(LoadedTreeState {
        commitment_tree: tree,
        last_block_height: r.last_block_height as u32,
        last_block_hash: r.last_block_hash,
    })
}

/// Persist the commitment tree, block height, and block hash to SQLite.
pub fn save_tree_state(
    tree: &BridgeTree<MerkleHashOrchard, u32, NOTE_COMMITMENT_TREE_DEPTH>,
    last_block_height: u32,
    last_block_hash: &str,
) {
    let database_url = database_url();
    let mut conn = establish_connection(&database_url);
    ensure_table(&mut conn);

    let json =
        serde_json::to_string(&serialize_tree(tree)).expect("failed to serialize commitment tree");

    let new_row = NewWalletStateRow {
        id: 1,
        commitment_tree_json: json,
        last_block_height: last_block_height as i32,
        last_block_hash: last_block_hash.to_string(),
    };

    let _ = conn.transaction(|conn| {
        use crate::schema::wallet_state::dsl as ws;
        diesel::delete(ws::wallet_state).execute(conn)?;
        diesel::insert_into(ws::wallet_state)
            .values(&new_row)
            .execute(conn)?;
        Ok::<_, diesel::result::Error>(())
    });
}

/// Delete the persisted tree state from SQLite.
/// Silently does nothing if the database is not reachable.
pub fn delete_tree_state() {
    let Some(database_url) = try_database_url() else {
        return;
    };
    let mut conn = establish_connection(&database_url);
    ensure_table(&mut conn);

    use crate::schema::wallet_state::dsl as ws;
    let _ = diesel::delete(ws::wallet_state).execute(&mut conn);
}

// ---------------------------------------------------------------------------
// Diesel row types
// ---------------------------------------------------------------------------

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::wallet_state)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct WalletStateRow {
    #[allow(dead_code)]
    id: i32,
    commitment_tree_json: String,
    last_block_height: i32,
    last_block_hash: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::wallet_state)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct NewWalletStateRow {
    id: i32,
    commitment_tree_json: String,
    last_block_height: i32,
    last_block_hash: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ensure_table(conn: &mut SqliteConnection) {
    let _ = sql_query(CREATE_TABLE_SQL).execute(conn);
}

fn establish_connection(database_url: &str) -> SqliteConnection {
    SqliteConnection::establish(database_url).expect("Error connecting to wallet_state database")
}

const DEFAULT_DATABASE_URL: &str = "walletdb.sqlite";

/// Returns the DATABASE_URL, or None if it is not set and the default DB file
/// does not exist. Used by load/delete which must not panic during init or reorg.
fn try_database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
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

fn database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::user::MAX_CHECKPOINTS;
    use tempfile::NamedTempFile;

    fn test_db_url() -> (NamedTempFile, String) {
        let db = NamedTempFile::new().unwrap();
        let url = db.path().to_string_lossy().to_string();
        (db, url)
    }

    #[test]
    fn test_empty_tree_roundtrip() {
        let tree: BridgeTree<MerkleHashOrchard, u32, NOTE_COMMITMENT_TREE_DEPTH> =
            BridgeTree::new(MAX_CHECKPOINTS);

        let ser = serialize_tree(&tree);
        let json = serde_json::to_string(&ser).unwrap();
        let deser: SerTree = serde_json::from_str(&json).unwrap();
        let restored = deserialize_tree(deser).unwrap();

        assert_eq!(tree, restored);
    }

    #[test]
    fn test_tree_with_leaves_roundtrip() {
        let mut tree: BridgeTree<MerkleHashOrchard, u32, NOTE_COMMITMENT_TREE_DEPTH> =
            BridgeTree::new(MAX_CHECKPOINTS);

        let leaf = MerkleHashOrchard::from_bytes(&[1u8; 32]).unwrap();
        tree.append(leaf);
        tree.mark();
        tree.checkpoint(0);

        let leaf2 = MerkleHashOrchard::from_bytes(&[2u8; 32]).unwrap();
        tree.append(leaf2);
        tree.checkpoint(1);

        let ser = serialize_tree(&tree);
        let json = serde_json::to_string(&ser).unwrap();
        let deser: SerTree = serde_json::from_str(&json).unwrap();
        let restored = deserialize_tree(deser).unwrap();

        assert_eq!(tree, restored);
    }

    #[test]
    fn test_db_save_load_delete() {
        let (_db, url) = test_db_url();
        let mut conn = establish_connection(&url);
        ensure_table(&mut conn);

        // No state initially
        {
            use crate::schema::wallet_state::dsl as ws;
            let rows: Vec<WalletStateRow> = ws::wallet_state
                .select(WalletStateRow::as_select())
                .load(&mut conn)
                .unwrap_or_default();
            assert!(rows.is_empty());
        }

        // Save
        let tree: BridgeTree<MerkleHashOrchard, u32, NOTE_COMMITMENT_TREE_DEPTH> =
            BridgeTree::new(MAX_CHECKPOINTS);
        let json = serde_json::to_string(&serialize_tree(&tree)).unwrap();
        let new_row = NewWalletStateRow {
            id: 1,
            commitment_tree_json: json,
            last_block_height: 42,
            last_block_hash: "abcdef".to_string(),
        };
        diesel::insert_into(crate::schema::wallet_state::table)
            .values(&new_row)
            .execute(&mut conn)
            .unwrap();

        // Load
        {
            use crate::schema::wallet_state::dsl as ws;
            let rows: Vec<WalletStateRow> = ws::wallet_state
                .select(WalletStateRow::as_select())
                .load(&mut conn)
                .unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].last_block_height, 42);
            assert_eq!(rows[0].last_block_hash, "abcdef");
        }

        // Delete
        {
            use crate::schema::wallet_state::dsl as ws;
            diesel::delete(ws::wallet_state).execute(&mut conn).unwrap();
            let rows: Vec<WalletStateRow> = ws::wallet_state
                .select(WalletStateRow::as_select())
                .load(&mut conn)
                .unwrap();
            assert!(rows.is_empty());
        }
    }
}
