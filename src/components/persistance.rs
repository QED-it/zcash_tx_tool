use crate::components::persistance::model::NoteData;

mod model;
mod sqlite;

trait DataStorage {
    fn find_notes(&self) -> Vec<NoteData>;
    fn insert_note(&self, note: NoteData) -> NoteData;
}