use std::env;
use diesel::associations::HasTable;
use diesel::prelude::*;
use dotenvy::dotenv;
use orchard::Address;
use orchard::note::{AssetBase, Nullifier};
use zcash_primitives::transaction::TxId;
use crate::components::persistence::model::{InsertableIssuanceData, InsertableNoteData, IssuanceData, NoteData};
use crate::schema::notes as note;
use crate::schema::notes::dsl::notes;
use crate::schema::issuance_data as issuance;
use crate::schema::issuance_data::dsl::issuance_data;

pub struct SqliteDataStorage {
    connection: SqliteConnection
}

impl SqliteDataStorage {
    pub fn new() -> Self {
        Self {
            connection: establish_connection()
        }
    }
}

impl SqliteDataStorage {
    // Notes

    pub fn find_notes(&mut self) -> Vec<NoteData> {
        notes
            .select(NoteData::as_select())
            .load(&mut self.connection)
            .expect("Error loading notes")
    }

    pub fn find_non_spent_notes(&mut self, recipient: Address) -> Vec<NoteData> {
        let criteria = note::spend_tx_id.is_null().and(
            note::recipient_address.eq(recipient.to_raw_address_bytes().to_vec())
        );

        notes
            .filter(criteria)
            .select(NoteData::as_select())
            .load(&mut self.connection)
            .expect("Error loading notes")
    }

    pub fn find_notes_for_tx(&mut self, txid: &TxId) -> Vec<NoteData> {
        notes
            .filter(note::tx_id.eq(txid.0.to_vec()))
            .select(NoteData::as_select())
            .load(&mut self.connection)
            .expect("Error loading notes")
    }

    pub fn find_by_nullifier(&mut self, nf: &Nullifier) -> Option<NoteData> {
        notes
            .filter(note::nullifier.eq(nf.to_bytes().to_vec()))
            .select(NoteData::as_select())
            .limit(1)
            .load(&mut self.connection)
            .expect("Error loading notes").pop()
    }

    pub fn mark_as_potentially_spent(&mut self, note_id: i32, spend_tx_id_value: &TxId, spend_action_index_value: i32) {
        diesel::update(notes)
            .filter(note::id.eq(note_id))
            .set((note::spend_tx_id.eq(spend_tx_id_value.0.to_vec()), note::spend_action_index.eq(spend_action_index_value)))
            .execute(&mut self.connection).unwrap();
    }

    pub fn update_note_position(&mut self, note_id: i32, position_value: i64) {
        diesel::update(notes)
            .filter(note::id.eq(note_id))
            .set(note::position.eq(position_value))
            .execute(&mut self.connection).unwrap();
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

    // Issued assets data

    pub fn find_issuance_data(&mut self) -> Vec<IssuanceData> {
        issuance_data
            .select(IssuanceData::as_select())
            .load(&mut self.connection)
            .expect("Error loading issuance data")
    }

    pub fn insert_issuance_data(&mut self, asset: &AssetBase, amount: i64, finalized: i32) -> IssuanceData {
        let entry = InsertableIssuanceData {
            amount,
            asset: asset.to_bytes().to_vec(),
            finalized
        };

        return diesel::insert_into(issuance_data::table())
            .values(entry)
            .returning(IssuanceData::as_returning())
            .get_result(&mut self.connection)
            .expect("Error saving new issuance data")
    }

    pub fn update_issuance_data(&mut self, id: i32, amount: i64, finalized: i32) {
        diesel::update(issuance_data)
            .filter(issuance::id.eq(id))
            .set((issuance::amount.eq(amount), issuance::finalized.eq(finalized)))
            .execute(&mut self.connection).unwrap();
    }

    pub fn find_issuance_data_for_asset(&mut self, asset: &AssetBase) -> Option<IssuanceData> {
        issuance_data
            .filter(issuance::asset.eq(asset.to_bytes().to_vec()))
            .select(IssuanceData::as_select())
            .limit(1)
            .load(&mut self.connection)
            .expect("Error loading issuance data").pop()
    }

    pub fn delete_all_issuance_data(&mut self) {
        diesel::delete(issuance_data)
            .execute(&mut self.connection)
            .expect("Error deleting issuance data");
    }
}

fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}