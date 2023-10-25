use diesel::prelude::*;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::notes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NoteData {
    pub id: i32,
    pub amount: i64,
 //   pub asset: AssetBase,
    pub tx_id: Vec<u8>,
    pub action_index: i32,
    pub merkle_path: Vec<u8>,
    pub encrypted_note: Vec<u8>,
    pub nullifier: Vec<u8>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::notes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct InsertableNoteData {
    pub amount: i64,
    //   pub asset: AssetBase,
    pub tx_id: Vec<u8>,
    pub action_index: i32,
    pub merkle_path: Vec<u8>,
    pub encrypted_note: Vec<u8>,
    pub nullifier: Vec<u8>,
}

impl InsertableNoteData {
    pub fn from_note_data(note: NoteData) -> Self {
        Self {
            amount: note.amount,
            tx_id: note.tx_id,
            action_index: note.action_index,
            merkle_path: note.merkle_path,
            encrypted_note: note.encrypted_note,
            nullifier: note.nullifier,
        }
    }
}