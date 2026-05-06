use crate::components::persistence::model::{InsertableNoteData, NoteData};
use crate::schema::notes::dsl::notes;
use crate::schema::notes::*;
use diesel::associations::HasTable;
use diesel::prelude::*;
use orchard::note::{AssetBase, Nullifier};
use orchard::Address;
use zcash_primitives::transaction::TxId;

/// Free functions taking `&mut SqliteConnection` so callers can wrap them in
/// a single transaction.
pub fn find_non_spent_notes(
    conn: &mut SqliteConnection,
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
        .load(conn)
        .expect("Error loading notes")
}

pub fn find_notes_for_tx(conn: &mut SqliteConnection, txid: &TxId) -> Vec<NoteData> {
    notes
        .filter(tx_id.eq(txid.as_ref().to_vec()))
        .select(NoteData::as_select())
        .load(conn)
        .expect("Error loading notes")
}

pub fn find_by_nullifier(conn: &mut SqliteConnection, nf: &Nullifier) -> Option<NoteData> {
    notes
        .filter(nullifier.eq(nf.to_bytes().to_vec()))
        .select(NoteData::as_select())
        .limit(1)
        .load(conn)
        .expect("Error loading notes")
        .pop()
}

pub fn mark_as_potentially_spent(
    conn: &mut SqliteConnection,
    note_id: i32,
    spend_tx_id_value: &TxId,
    spend_action_index_value: i32,
) {
    diesel::update(notes)
        .filter(id.eq(note_id))
        .set((
            spend_tx_id.eq(spend_tx_id_value.as_ref().to_vec()),
            spend_action_index.eq(spend_action_index_value),
        ))
        .execute(conn)
        .unwrap();
}

pub fn update_note_position(conn: &mut SqliteConnection, note_id: i32, position_value: i64) {
    diesel::update(notes)
        .filter(id.eq(note_id))
        .set(position.eq(position_value))
        .execute(conn)
        .unwrap();
}

pub fn insert_note(conn: &mut SqliteConnection, note: NoteData) -> NoteData {
    diesel::insert_into(notes::table())
        .values(&InsertableNoteData::from_note_data(note))
        .returning(NoteData::as_returning())
        .get_result(conn)
        .expect("Error saving new note")
}

pub fn delete_all_notes(conn: &mut SqliteConnection) {
    diesel::delete(notes)
        .execute(conn)
        .expect("Error deleting notes");
}
