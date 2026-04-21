//! SQLite-backed persistence for the note commitment tree and sync position.

use bridgetree::{BridgeTree, Checkpoint, MerkleBridge};
use diesel::prelude::*;
use incrementalmerkletree::frontier::NonEmptyFrontier;
use incrementalmerkletree::{Address, Level, Position};
use orchard::tree::MerkleHashOrchard;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::components::db;
use crate::components::user::NOTE_COMMITMENT_TREE_DEPTH;

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

impl From<&BridgeTree<MerkleHashOrchard, u32, NOTE_COMMITMENT_TREE_DEPTH>> for SerTree {
    fn from(tree: &BridgeTree<MerkleHashOrchard, u32, NOTE_COMMITMENT_TREE_DEPTH>) -> Self {
        Self {
            prior_bridges: tree.prior_bridges().iter().map(SerBridge::from).collect(),
            current_bridge: tree.current_bridge().as_ref().map(SerBridge::from),
            saved: tree
                .marked_indices()
                .iter()
                .map(|(pos, idx)| (u64::from(*pos), *idx))
                .collect(),
            checkpoints: tree
                .checkpoints()
                .iter()
                .map(SerCheckpoint::from)
                .collect(),
            max_checkpoints: tree.max_checkpoints(),
        }
    }
}

impl From<&MerkleBridge<MerkleHashOrchard>> for SerBridge {
    fn from(bridge: &MerkleBridge<MerkleHashOrchard>) -> Self {
        Self {
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
            frontier: SerFrontier::from(bridge.frontier()),
        }
    }
}

impl From<&NonEmptyFrontier<MerkleHashOrchard>> for SerFrontier {
    fn from(frontier: &NonEmptyFrontier<MerkleHashOrchard>) -> Self {
        Self {
            position: u64::from(frontier.position()),
            leaf: frontier.leaf().to_bytes(),
            ommers: frontier.ommers().iter().map(|h| h.to_bytes()).collect(),
        }
    }
}

impl From<&Checkpoint<u32>> for SerCheckpoint {
    fn from(cp: &Checkpoint<u32>) -> Self {
        Self {
            id: *cp.id(),
            bridges_len: cp.bridges_len(),
            marked: cp.marked().iter().map(|p| u64::from(*p)).collect(),
            forgotten: cp.forgotten().iter().map(|p| u64::from(*p)).collect(),
        }
    }
}

impl TryFrom<SerTree> for BridgeTree<MerkleHashOrchard, u32, NOTE_COMMITMENT_TREE_DEPTH> {
    type Error = String;

    fn try_from(ser: SerTree) -> Result<Self, Self::Error> {
        let prior_bridges: Vec<MerkleBridge<MerkleHashOrchard>> = ser
            .prior_bridges
            .into_iter()
            .map(MerkleBridge::try_from)
            .collect::<Result<_, _>>()?;

        let current_bridge = ser
            .current_bridge
            .map(MerkleBridge::try_from)
            .transpose()?;

        let saved: BTreeMap<Position, usize> = ser
            .saved
            .into_iter()
            .map(|(pos, idx)| (Position::from(pos), idx))
            .collect();

        let checkpoints: VecDeque<Checkpoint<u32>> = ser
            .checkpoints
            .into_iter()
            .map(Checkpoint::try_from)
            .collect::<Result<_, _>>()?;

        BridgeTree::from_parts(
            prior_bridges,
            current_bridge,
            saved,
            checkpoints,
            ser.max_checkpoints,
        )
        .map_err(|e| format!("BridgeTree::from_parts failed: {:?}", e))
    }
}

impl TryFrom<SerBridge> for MerkleBridge<MerkleHashOrchard> {
    type Error = String;

    fn try_from(ser: SerBridge) -> Result<Self, Self::Error> {
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

        let frontier = NonEmptyFrontier::try_from(ser.frontier)?;

        Ok(MerkleBridge::from_parts(
            prior_position,
            tracking,
            ommers,
            frontier,
        ))
    }
}

impl TryFrom<SerFrontier> for NonEmptyFrontier<MerkleHashOrchard> {
    type Error = String;

    fn try_from(ser: SerFrontier) -> Result<Self, Self::Error> {
        let position = Position::from(ser.position);
        let leaf = Option::from(MerkleHashOrchard::from_bytes(&ser.leaf))
            .ok_or("invalid frontier leaf hash")?;
        let ommers: Vec<MerkleHashOrchard> = ser
            .ommers
            .into_iter()
            .map(|bytes| {
                Option::from(MerkleHashOrchard::from_bytes(&bytes))
                    .ok_or("invalid frontier ommer hash")
            })
            .collect::<Result<_, _>>()?;

        NonEmptyFrontier::from_parts(position, leaf, ommers)
            .map_err(|e| format!("NonEmptyFrontier::from_parts failed: {:?}", e))
    }
}

impl TryFrom<SerCheckpoint> for Checkpoint<u32> {
    type Error = String;

    fn try_from(ser: SerCheckpoint) -> Result<Self, Self::Error> {
        Ok(Checkpoint::from_parts(
            ser.id,
            ser.bridges_len,
            ser.marked.into_iter().map(Position::from).collect(),
            ser.forgotten.into_iter().map(Position::from).collect(),
        ))
    }
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
/// Returns `Ok(None)` if no state is stored or the database does not exist.
pub fn load_tree_state() -> Result<Option<LoadedTreeState>, String> {
    let Some(database_url) = db::try_database_url() else {
        return Ok(None);
    };
    let mut conn = db::establish_connection(&database_url);

    use crate::schema::wallet_state::dsl as ws;
    let row = ws::wallet_state
        .select(WalletStateRow::as_select())
        .first(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to query wallet_state: {e}"))?;

    let Some(r) = row else {
        return Ok(None);
    };

    let ser_tree: SerTree = serde_json::from_str(&r.commitment_tree_json)
        .map_err(|e| format!("Failed to deserialize saved tree state: {e}"))?;
    let tree = BridgeTree::try_from(ser_tree)?;
    Ok(Some(LoadedTreeState {
        commitment_tree: tree,
        last_block_height: r.last_block_height as u32,
        last_block_hash: r.last_block_hash,
    }))
}

/// Persist the commitment tree, block height, and block hash to SQLite.
pub fn save_tree_state(
    tree: &BridgeTree<MerkleHashOrchard, u32, NOTE_COMMITMENT_TREE_DEPTH>,
    last_block_height: u32,
    last_block_hash: &str,
) -> Result<(), String> {
    let database_url = db::database_url();
    let mut conn = db::establish_connection(&database_url);

    let ser_tree = SerTree::from(tree);
    let json = serde_json::to_string(&ser_tree)
        .map_err(|e| format!("Failed to serialize commitment tree: {e}"))?;

    let new_row = WalletStateRow {
        id: 1,
        commitment_tree_json: json,
        last_block_height: last_block_height as i32,
        last_block_hash: last_block_hash.to_string(),
    };

    conn.transaction(|conn| {
        use crate::schema::wallet_state::dsl as ws;
        diesel::delete(ws::wallet_state).execute(conn)?;
        diesel::insert_into(ws::wallet_state)
            .values(&new_row)
            .execute(conn)?;
        Ok::<_, diesel::result::Error>(())
    })
    .map_err(|e| format!("Failed to save tree state: {e}"))
}

/// Delete the persisted tree state from SQLite.
/// Does nothing if the database does not exist.
pub fn delete_tree_state() -> Result<(), String> {
    let Some(database_url) = db::try_database_url() else {
        return Ok(());
    };
    let mut conn = db::establish_connection(&database_url);

    use crate::schema::wallet_state::dsl as ws;
    diesel::delete(ws::wallet_state)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to delete tree state: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Diesel row types
// ---------------------------------------------------------------------------

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::wallet_state)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct WalletStateRow {
    id: i32,
    commitment_tree_json: String,
    last_block_height: i32,
    last_block_hash: String,
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

        let ser = SerTree::from(&tree);
        let json = serde_json::to_string(&ser).unwrap();
        let deser: SerTree = serde_json::from_str(&json).unwrap();
        let restored = BridgeTree::try_from(deser).unwrap();

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

        let ser = SerTree::from(&tree);
        let json = serde_json::to_string(&ser).unwrap();
        let deser: SerTree = serde_json::from_str(&json).unwrap();
        let restored = BridgeTree::try_from(deser).unwrap();

        assert_eq!(tree, restored);
    }

    #[test]
    fn test_db_save_load_delete() {
        let (_db, url) = test_db_url();
        let mut conn = db::establish_connection(&url);

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
        let json = serde_json::to_string(&SerTree::from(&tree)).unwrap();
        let new_row = WalletStateRow {
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
