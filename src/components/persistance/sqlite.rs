use std::env;
use diesel::associations::HasTable;
use diesel::prelude::*;
use dotenvy::dotenv;
use crate::components::persistance::DataStorage;
use crate::components::persistance::model::NoteData;
use crate::schema::notes::dsl::notes;

struct SqliteDataStorage {
    // TODO connection management
    connection: SqliteConnection
}

impl SqliteDataStorage {
    fn new() -> Self {
        Self {
            connection: establish_connection()
        }
    }
}

impl DataStorage for SqliteDataStorage {
    fn find_notes(&self) -> Vec<NoteData> {
        let connection = &mut establish_connection();

        notes
            .select(NoteData::as_select())
            .load(connection)
            .expect("Error loading notes")
    }

    fn insert_note(&self, note: NoteData) -> NoteData {
        let connection = &mut establish_connection();

        // TODO? let new_post = InsertNote { note_field1, note_field2 };

        diesel::insert_into(notes::table())
            .values(&note)
            .returning(NoteData::as_returning())
            .get_result(connection)
            .expect("Error saving new note")
    }
}

fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}