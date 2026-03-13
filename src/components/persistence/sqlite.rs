use abscissa_core::prelude::info;
use crate::components::persistence::model::{InsertableNoteData, NoteData};
use crate::schema::notes::dsl::notes;
use crate::schema::notes::*;
use diesel::associations::HasTable;
use diesel::prelude::*;
use dotenvy::dotenv;
use orchard::note::{AssetBase, Nullifier};
use orchard::Address;
use std::env;
use zcash_primitives::transaction::TxId;

pub struct SqliteDataStorage {
    connection: SqliteConnection,
}

impl SqliteDataStorage {
    pub fn new() -> Self {
        Self {
            connection: establish_connection(),
        }
    }
}

impl Default for SqliteDataStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl SqliteDataStorage {
    pub fn find_notes(&mut self) -> Vec<NoteData> {
        notes
            .select(NoteData::as_select())
            .load(&mut self.connection)
            .expect("Error loading notes")
    }

    pub fn find_non_spent_notes(
        &mut self,
        recipient: Address,
        asset_base: AssetBase,
    ) -> Vec<NoteData> {
        notes
            .filter(
                spend_tx_id
                    .is_null()
                    .and(recipient_address.eq(recipient.to_raw_address_bytes().to_vec()))
                    .and(asset.eq(asset_base.to_bytes().to_vec())),
            )
            .select(NoteData::as_select())
            .load(&mut self.connection)
            .expect("Error loading notes")
    }

    pub fn find_notes_for_tx(&mut self, txid: &TxId) -> Vec<NoteData> {
        notes
            .filter(tx_id.eq(txid.as_ref().to_vec()))
            .select(NoteData::as_select())
            .load(&mut self.connection)
            .expect("Error loading notes")
    }

    pub fn find_by_nullifier(&mut self, nf: &Nullifier) -> Option<NoteData> {
        notes
            .filter(nullifier.eq(nf.to_bytes().to_vec()))
            .select(NoteData::as_select())
            .limit(1)
            .load(&mut self.connection)
            .expect("Error loading notes")
            .pop()
    }

    pub fn mark_as_potentially_spent(
        &mut self,
        note_id: i32,
        spend_tx_id_value: &TxId,
        spend_action_index_value: i32,
        spend_block_height_value: i32,
    ) {
        diesel::update(notes)
            .filter(id.eq(note_id))
            .set((
                spend_tx_id.eq(spend_tx_id_value.as_ref().to_vec()),
                spend_action_index.eq(spend_action_index_value),
                spend_block_height.eq(spend_block_height_value),
            ))
            .execute(&mut self.connection)
            .unwrap();
    }

    pub fn update_note_position(&mut self, note_id: i32, position_value: i64) {
        diesel::update(notes)
            .filter(id.eq(note_id))
            .set(position.eq(position_value))
            .execute(&mut self.connection)
            .unwrap();
    }

    pub fn insert_note(&mut self, note: NoteData) -> NoteData {
        diesel::insert_into(notes::table())
            .values(&InsertableNoteData::from_note_data(note))
            .returning(NoteData::as_returning())
            .get_result(&mut self.connection)
            .expect("Error saving new note")
    }

    pub fn delete_all_notes(&mut self) {
        diesel::delete(notes)
            .execute(&mut self.connection)
            .expect("Error deleting notes");
    }

    /// Handle blockchain reorganization by cleaning up invalidated note data.
    /// - Deletes notes that were created in blocks at or after `reorg_height`
    /// - Clears spend info for notes that were spent in blocks at or after `reorg_height`
    ///   (but keeps the note itself if it was created before the reorg point)
    pub fn handle_reorg(&mut self, reorg_height: i32) {
        // Delete notes created in invalidated blocks
        let deleted = diesel::delete(notes.filter(origin_block_height.ge(reorg_height)))
            .execute(&mut self.connection)
            .expect("Error deleting notes from reorged blocks");

        if deleted > 0 {
            info!(
                "Reorg: deleted {} notes created at or after height {}",
                deleted, reorg_height
            );
        }

        // Clear spend info for notes that were spent in invalidated blocks
        // but were created before the reorg point (so they're still valid, just unspent now)
        let updated = diesel::update(
            notes.filter(
                spend_block_height
                    .ge(reorg_height)
                    .and(origin_block_height.lt(reorg_height)),
            ),
        )
        .set((
            spend_tx_id.eq(None::<Vec<u8>>),
            spend_block_height.eq(None::<i32>),
        ))
        .execute(&mut self.connection)
        .expect("Error clearing spend info from reorged blocks");

        if updated > 0 {
            info!(
                "Reorg: cleared spend info for {} notes spent at or after height {}",
                updated, reorg_height
            );
        }
    }
}

fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
