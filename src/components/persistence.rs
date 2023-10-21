use crate::components::persistence::model::RawNoteData;

mod model;
mod sqlite;

trait DataStorage {
    fn find_notes(&self) -> Vec<RawNoteData>;
    fn insert_note(&self, note: RawNoteData) -> RawNoteData;
}