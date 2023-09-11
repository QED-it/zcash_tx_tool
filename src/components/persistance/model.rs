use diesel::prelude::*;
use orchard::note::AssetBase;
use orchard::tree::MerklePath;
use zcash_primitives::transaction::components::Amount;
use zcash_primitives::transaction::TxId;
use zebra_chain::orchard::EncryptedNote;

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::notes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NoteData {
    pub id: i32,
    pub amount: Amount,
    pub asset: AssetBase,
    pub tx_id: TxId,
    pub tx_index: u32,
    pub merkle_path: MerklePath,
    pub encrypted_note: EncryptedNote
}