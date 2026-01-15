use rusqlite::{Connection, Result};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct SessionData {
    pub id: u64,
    pub created_at: u64,
    pub updated_at: u64,
    pub deck_name: String,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub questions_total: usize,
    pub questions_answered: usize,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn create_session(conn: &Connection, deck_name: &str, questions_total: usize) -> Result<u64> {
    let created_at = now();
    let updated_at = created_at;
    let started_at = created_at;

    conn.execute(
        "INSERT INTO sessions (created_at, updated_at, deck_name, started_at, questions_total, questions_answered)
         VALUES (?, ?, ?, ?, ?, 0)",
        rusqlite::params![created_at, updated_at, deck_name, started_at, questions_total],
    )?;

    Ok(conn.last_insert_rowid() as u64)
}

pub fn get_session(conn: &Connection, id: u64) -> Result<Option<SessionData>> {
    let mut stmt = conn.prepare(
        "SELECT id, created_at, updated_at, deck_name, started_at, completed_at, questions_total, questions_answered
         FROM sessions WHERE id = ?",
    )?;

    stmt.query_row([id], |row| {
        Ok(SessionData {
            id: row.get(0)?,
            created_at: row.get(1)?,
            updated_at: row.get(2)?,
            deck_name: row.get(3)?,
            started_at: row.get(4)?,
            completed_at: row.get(5)?,
            questions_total: row.get(6)?,
            questions_answered: row.get(7)?,
        })
    })
    .map(Some)
    .or(Ok(None))
}

pub fn update_progress(conn: &Connection, session_id: u64, answered: usize) -> Result<()> {
    let updated_at = now();
    conn.execute(
        "UPDATE sessions SET updated_at = ?, questions_answered = ? WHERE id = ?",
        rusqlite::params![updated_at, answered, session_id],
    )?;
    Ok(())
}

pub fn complete_session(conn: &Connection, session_id: u64) -> Result<()> {
    let updated_at = now();
    let completed_at = updated_at;
    conn.execute(
        "UPDATE sessions SET updated_at = ?, completed_at = ? WHERE id = ?",
        rusqlite::params![updated_at, completed_at, session_id],
    )?;
    Ok(())
}

pub fn session_exists(conn: &Connection, session_id: u64) -> bool {
    conn.query_row("SELECT 1 FROM sessions WHERE id = ?", [session_id], |_| {
        Ok(())
    })
    .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::run_migrations;

    #[test]
    fn test_create_and_get_session() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_db_path = temp_dir.path().join("test.db");
        let mut conn = Connection::open(&test_db_path).unwrap();
        run_migrations(&mut conn).unwrap();

        let session_id = create_session(&conn, "Test Deck", 10).unwrap();
        assert_eq!(session_id, 1);

        let session = get_session(&conn, session_id).unwrap().unwrap();
        assert_eq!(session.deck_name, "Test Deck");
        assert_eq!(session.questions_total, 10);
        assert_eq!(session.questions_answered, 0);
        assert!(session.completed_at.is_none());
    }

    #[test]
    fn test_update_progress() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_db_path = temp_dir.path().join("test.db");
        let mut conn = Connection::open(&test_db_path).unwrap();
        run_migrations(&mut conn).unwrap();

        let session_id = create_session(&conn, "Test Deck", 10).unwrap();
        update_progress(&conn, session_id, 5).unwrap();

        let session = get_session(&conn, session_id).unwrap().unwrap();
        assert_eq!(session.questions_answered, 5);
    }

    #[test]
    fn test_complete_session() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_db_path = temp_dir.path().join("test.db");
        let mut conn = Connection::open(&test_db_path).unwrap();
        run_migrations(&mut conn).unwrap();

        let session_id = create_session(&conn, "Test Deck", 10).unwrap();
        complete_session(&conn, session_id).unwrap();

        let session = get_session(&conn, session_id).unwrap().unwrap();
        assert!(session.completed_at.is_some());
    }

    #[test]
    fn test_get_nonexistent_session() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_db_path = temp_dir.path().join("test.db");
        let mut conn = Connection::open(&test_db_path).unwrap();
        run_migrations(&mut conn).unwrap();

        let session = get_session(&conn, 999).unwrap();
        assert!(session.is_none());
    }
}
