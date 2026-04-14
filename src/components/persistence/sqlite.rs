use crate::components::db;
use crate::components::persistence::model::{InsertableNoteData, NoteData};
use crate::schema::notes::dsl::notes;
use crate::schema::notes::*;
use diesel::associations::HasTable;
use diesel::prelude::*;
use orchard::note::{AssetBase, Nullifier};
use orchard::Address;
use zcash_primitives::transaction::TxId;

pub struct SqliteDataStorage {
    connection: SqliteConnection,
}

impl SqliteDataStorage {
    pub fn new() -> Self {
        Self {
            connection: db::establish_connection(&db::database_url()),
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

}
