use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::notes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NoteData {
    // Skip on insert so SQLite auto-assigns the rowid-aliased primary key;
    // otherwise Diesel sends the literal `id = 0` and the second row collides.
    #[diesel(skip_insertion)]
    pub id: i32,
    pub amount: i64,
    pub asset: Vec<u8>,
    pub tx_id: Vec<u8>,
    pub action_index: i32,
    pub position: i64,
    pub memo: Vec<u8>,
    pub rho: Vec<u8>,
    pub nullifier: Vec<u8>,
    pub rseed: Vec<u8>,
    pub recipient_address: Vec<u8>,
    pub spend_tx_id: Option<Vec<u8>>,
    pub spend_action_index: i32,
    pub origin_block_height: i32,
    pub spend_block_height: Option<i32>,
}
