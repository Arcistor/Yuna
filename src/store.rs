use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::types::{EventKind, EventRecord, MoodState, NoteRecord};

#[derive(Debug, Clone)]
pub struct Store {
    db_path: PathBuf,
}

impl Store {
    pub fn new(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create database directory {}", parent.display()))?;
        }
        let store = Self {
            db_path: path.to_path_buf(),
        };
        store.init()?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.db_path
    }

    fn connect(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
            .with_context(|| format!("open database {}", self.db_path.display()))
    }

    fn init(&self) -> Result<()> {
        let conn = self.connect()?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS events (
              id        INTEGER PRIMARY KEY,
              path      TEXT NOT NULL,
              kind      TEXT NOT NULL,
              timestamp INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS mood (
              id        INTEGER PRIMARY KEY CHECK (id = 1),
              state     TEXT NOT NULL,
              updated   INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS notes (
              id        INTEGER PRIMARY KEY,
              path      TEXT NOT NULL,
              trigger   TEXT NOT NULL,
              created   INTEGER NOT NULL,
              read_at   INTEGER,
              deleted   INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS silence (
              id        INTEGER PRIMARY KEY CHECK (id = 1),
              until     INTEGER NOT NULL
            );

            INSERT OR IGNORE INTO mood (id, state, updated) VALUES (1, 'calm', 0);
            CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
            CREATE INDEX IF NOT EXISTS idx_events_kind_timestamp ON events(kind, timestamp);
            CREATE INDEX IF NOT EXISTS idx_notes_trigger_created ON notes(trigger, created);
            "#,
        )
        .context("initialize database schema")?;
        Ok(())
    }

    pub fn insert_event(&self, path: &Path, kind: EventKind, timestamp: i64) -> Result<()> {
        self.connect()?.execute(
            "INSERT INTO events (path, kind, timestamp) VALUES (?1, ?2, ?3)",
            params![path.to_string_lossy(), kind.as_str(), timestamp],
        )?;
        Ok(())
    }

    pub fn query_events(&self, since: i64, kind: Option<EventKind>) -> Result<Vec<EventRecord>> {
        let conn = self.connect()?;
        let mut events = Vec::new();

        if let Some(kind) = kind {
            let mut stmt = conn.prepare(
                "SELECT id, path, kind, timestamp FROM events
                 WHERE timestamp >= ?1 AND kind = ?2
                 ORDER BY timestamp ASC",
            )?;
            let rows = stmt.query_map(params![since, kind.as_str()], map_event)?;
            for row in rows {
                events.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, path, kind, timestamp FROM events
                 WHERE timestamp >= ?1
                 ORDER BY timestamp ASC",
            )?;
            let rows = stmt.query_map(params![since], map_event)?;
            for row in rows {
                events.push(row?);
            }
        }

        Ok(events)
    }

    pub fn get_mood(&self) -> Result<MoodState> {
        let state: String =
            self.connect()?
                .query_row("SELECT state FROM mood WHERE id = 1", [], |row| row.get(0))?;
        MoodState::from_str(&state)
    }

    pub fn set_mood(&self, state: MoodState) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.connect()?.execute(
            "INSERT INTO mood (id, state, updated) VALUES (1, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET state = excluded.state, updated = excluded.updated",
            params![state.as_str(), now],
        )?;
        Ok(())
    }

    pub fn insert_note(&self, path: &Path, trigger: &str, created: i64) -> Result<i64> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO notes (path, trigger, created) VALUES (?1, ?2, ?3)",
            params![path.to_string_lossy(), trigger, created],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn mark_note_read(&self, id: i64, read_at: i64) -> Result<()> {
        self.connect()?.execute(
            "UPDATE notes SET read_at = ?1 WHERE id = ?2",
            params![read_at, id],
        )?;
        Ok(())
    }

    pub fn mark_note_deleted(&self, id: i64) -> Result<()> {
        self.connect()?
            .execute("UPDATE notes SET deleted = 1 WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn list_undeleted_notes(&self) -> Result<Vec<NoteRecord>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, path, trigger, created, read_at, deleted FROM notes
             WHERE deleted = 0
             ORDER BY created ASC",
        )?;
        let rows = stmt.query_map([], map_note)?;
        let mut notes = Vec::new();
        for row in rows {
            notes.push(row?);
        }
        Ok(notes)
    }

    pub fn recent_note_exists(&self, trigger: &str, since: i64) -> Result<bool> {
        let count: i64 = self.connect()?.query_row(
            "SELECT COUNT(*) FROM notes WHERE trigger = ?1 AND created >= ?2",
            params![trigger, since],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn recent_events(&self, limit: usize) -> Result<Vec<EventRecord>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, path, kind, timestamp FROM events
             ORDER BY timestamp DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![limit as i64], map_event)?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    pub fn last_event_time(&self) -> Result<Option<i64>> {
        Ok(self.connect()?
            .query_row("SELECT MAX(timestamp) FROM events", [], |row| row.get(0))?)
    }

    pub fn set_silenced_until(&self, until: i64) -> Result<()> {
        self.connect()?.execute(
            "INSERT INTO silence (id, until) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET until = excluded.until",
            params![until],
        )?;
        Ok(())
    }

    pub fn is_silenced(&self, now: i64) -> Result<bool> {
        let until = self
            .connect()?
            .query_row("SELECT until FROM silence WHERE id = 1", [], |row| {
                row.get::<_, i64>(0)
            })
            .optional()?;
        Ok(until.is_some_and(|value| value > now))
    }
}

fn map_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<EventRecord> {
    let kind: String = row.get(2)?;
    Ok(EventRecord {
        id: row.get(0)?,
        path: PathBuf::from(row.get::<_, String>(1)?),
        kind: EventKind::from_str(&kind).map_err(to_sql_error)?,
        timestamp: row.get(3)?,
    })
}

fn map_note(row: &rusqlite::Row<'_>) -> rusqlite::Result<NoteRecord> {
    let deleted: i64 = row.get(5)?;
    Ok(NoteRecord {
        id: row.get(0)?,
        path: PathBuf::from(row.get::<_, String>(1)?),
        trigger: row.get(2)?,
        created: row.get(3)?,
        read_at: row.get(4)?,
        deleted: deleted != 0,
    })
}

fn to_sql_error(error: anyhow::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error.to_string(),
        )),
    )
}
