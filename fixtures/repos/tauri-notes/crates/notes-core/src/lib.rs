use std::path::Path;

use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize)]
pub struct Note {
    pub id: i64,
    pub text: String,
}

pub struct NoteStore {
    conn: Connection,
}

impl NoteStore {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("CREATE TABLE IF NOT EXISTS notes (id INTEGER PRIMARY KEY, text TEXT NOT NULL);")?;
        Ok(Self { conn })
    }

    pub fn add(&mut self, text: &str) -> rusqlite::Result<Note> {
        self.conn.execute("INSERT INTO notes (text) VALUES (?1)", [text])?;
        Ok(Note { id: self.conn.last_insert_rowid(), text: text.to_owned() })
    }

    pub fn all(&self) -> rusqlite::Result<Vec<Note>> {
        let mut st = self.conn.prepare("SELECT id, text FROM notes ORDER BY id")?;
        let rows = st.query_map([], |r| Ok(Note { id: r.get(0)?, text: r.get(1)? }))?;
        rows.collect()
    }

    pub fn remove(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM notes WHERE id = ?1", [id])?;
        Ok(())
    }
}
