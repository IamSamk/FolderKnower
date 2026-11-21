//! `ark-store`: content-addressed SQLite cache.
//!
//! An artifact key is `blake3(file) | grammar@version | extractor_version`, so an
//! unchanged file is never re-parsed and a grammar/extractor bump invalidates
//! exactly the affected entries.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

const MIGRATIONS: &[&str] = &[
    // v1
    "CREATE TABLE IF NOT EXISTS files (path TEXT PRIMARY KEY, hash TEXT NOT NULL);
     CREATE TABLE IF NOT EXISTS artifacts (key TEXT PRIMARY KEY, blob BLOB NOT NULL);
     CREATE TABLE IF NOT EXISTS meta (k TEXT PRIMARY KEY, v TEXT NOT NULL);",
];

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
}

pub struct Store {
    conn: Connection,
}

/// Build the artifact key for one file.
pub fn artifact_key(content_hash: &str, grammar: &str, extractor_version: &str) -> String {
    format!("{content_hash}|{grammar}|{extractor_version}")
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    pub fn in_memory() -> Result<Self, StoreError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self, StoreError> {
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        conn.execute_batch("CREATE TABLE IF NOT EXISTS migrations (version INTEGER PRIMARY KEY);")?;
        let current: i64 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM migrations",
            [],
            |r| r.get(0),
        )?;
        for (i, sql) in MIGRATIONS.iter().enumerate() {
            let v = i as i64 + 1;
            if v > current {
                conn.execute_batch(sql)?;
                conn.execute("INSERT INTO migrations(version) VALUES (?1)", [v])?;
            }
        }
        Ok(Self { conn })
    }

    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT blob FROM artifacts WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    /// Insert many artifacts in one transaction.
    pub fn put_many<'a>(
        &mut self,
        items: impl IntoIterator<Item = (&'a str, &'a [u8])>,
    ) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt =
                tx.prepare("INSERT OR REPLACE INTO artifacts(key, blob) VALUES (?1, ?2)")?;
            for (k, v) in items {
                stmt.execute(params![k, v])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Record the current file set (path -> hash), replacing the previous one.
    pub fn set_files<'a>(
        &mut self,
        files: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM files", [])?;
        {
            let mut stmt = tx.prepare("INSERT INTO files(path, hash) VALUES (?1, ?2)")?;
            for (p, h) in files {
                stmt.execute(params![p, h])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn file_count(&self) -> Result<u64, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM files", [], |r| r.get::<_, i64>(0))? as u64)
    }

    /// Drop artifacts no longer referenced by any key in `live`.
    pub fn retain(&mut self, live: &[String]) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        tx.execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS live(key TEXT PRIMARY KEY); DELETE FROM live;",
        )?;
        {
            let mut stmt = tx.prepare("INSERT OR IGNORE INTO live(key) VALUES (?1)")?;
            for k in live {
                stmt.execute([k])?;
            }
        }
        tx.execute(
            "DELETE FROM artifacts WHERE key NOT IN (SELECT key FROM live)",
            [],
        )?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_retain() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(&dir.path().join("i.sqlite")).unwrap();
        s.put_many([("a", b"1".as_slice()), ("b", b"2".as_slice())])
            .unwrap();
        assert_eq!(s.get("a").unwrap().as_deref(), Some(b"1".as_slice()));
        s.retain(&["b".to_owned()]).unwrap();
        assert!(s.get("a").unwrap().is_none());
        s.set_files([("x.ts", "h")]).unwrap();
        assert_eq!(s.file_count().unwrap(), 1);
        drop(s);
        let s = Store::open(&dir.path().join("i.sqlite")).unwrap();
        assert_eq!(s.get("b").unwrap().as_deref(), Some(b"2".as_slice()));
    }
}
