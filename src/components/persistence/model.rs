use std::convert::TryInto;
use diesel::prelude::*;
use orchard::{Address, Note};
use orchard::note::{AssetBase, RandomSeed, Rho};
use orchard::value::NoteValue;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::notes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NoteData {
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
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::notes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct InsertableNoteData {
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
}

impl InsertableNoteData {
    pub fn from_note_data(note: NoteData) -> Self {
        Self {
            amount: note.amount,
            asset: note.asset,
            tx_id: note.tx_id,
            action_index: note.action_index,
            position: note.position,
            memo: note.memo,
            rho: note.rho,
            nullifier: note.nullifier,
            rseed: note.rseed,
            recipient_address: note.recipient_address,
            spend_tx_id: note.spend_tx_id,
            spend_action_index: note.spend_action_index,
        }
    }
}

impl From<NoteData> for Note {
    fn from(data: NoteData) -> Self {
        let rho = Rho::from_bytes(data.rho.as_slice().try_into().unwrap()).unwrap();

        Note::from_parts(
            Address::from_raw_address_bytes(data.recipient_address.as_slice().try_into().unwrap())
                .unwrap(),
            NoteValue::from_raw(data.amount as u64),
            AssetBase::from_bytes(data.asset.as_slice().try_into().unwrap()).unwrap(),
            rho,
            RandomSeed::from_bytes(data.rseed.as_slice().try_into().unwrap(), &rho).unwrap(),
        )
        .unwrap()
    }
}
