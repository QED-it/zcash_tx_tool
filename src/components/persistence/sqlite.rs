use std::env;
use diesel::associations::HasTable;
use diesel::prelude::*;
use dotenvy::dotenv;
use crate::components::persistence::model::{InsertableNoteData, NoteData};
use crate::schema::notes::amount;
use crate::schema::notes::dsl::notes;

pub struct SqliteDataStorage {
    // TODO connection management
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
    pub fn find_notes(&mut self) -> Vec<NoteData> {
        notes
            // .filter(
            //     amount.gt(0).and(
            //         amount.lt(1000))
            // )
            .select(NoteData::as_select())
            .load(&mut self.connection)
            .expect("Error loading notes")
    }

    pub fn insert_note(&mut self, note: NoteData) -> NoteData {
        diesel::insert_into(notes::table())
            .values(&InsertableNoteData::from_note_data(note))
            .returning(NoteData::as_returning())
            .get_result(&mut self.connection)
            .expect("Error saving new note")
    }
}

fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}