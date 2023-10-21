use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::notes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct RawNoteData {
    pub id: i32,
    pub amount: i64,
 //   pub asset: AssetBase,
    pub tx_id: Vec<u8>,
    pub tx_index: i32,
    pub merkle_path: Vec<u8>,
    pub encrypted_note: Vec<u8>
}