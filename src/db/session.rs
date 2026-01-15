use rusqlite::{Connection, Result};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::flashcard::{load_flashcards, FlashcardData};

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub id: u64,
    pub deck_name: String,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub questions_total: usize,
    pub questions_answered: usize,
}

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

pub fn list_sessions(conn: &Connection) -> Result<Vec<SessionSummary>> {
    let mut stmt = conn.prepare(
        "SELECT id, deck_name, started_at, completed_at, questions_total, questions_answered
         FROM sessions ORDER BY id DESC",
    )?;

    let sessions = stmt
        .query_map([], |row| {
            Ok(SessionSummary {
                id: row.get(0)?,
                deck_name: row.get(1)?,
                started_at: row.get(2)?,
                completed_at: row.get(3)?,
                questions_total: row.get(4)?,
                questions_answered: row.get(5)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(sessions)
}

pub fn get_session_detail(
    conn: &Connection,
    session_id: u64,
) -> Result<Option<(SessionData, Vec<FlashcardData>)>> {
    let session = get_session(conn, session_id)?;
    match session {
        Some(s) => {
            let flashcards = load_flashcards(conn, session_id)?;
            Ok(Some((s, flashcards)))
        }
        None => Ok(None),
    }
}

pub fn delete_session(conn: &Connection, session_id: u64) -> Result<()> {
    conn.execute("DELETE FROM flashcards WHERE session_id = ?", [session_id])?;
    conn.execute("DELETE FROM sessions WHERE id = ?", [session_id])?;
    Ok(())
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

    #[test]
    fn test_list_sessions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_db_path = temp_dir.path().join("test.db");
        let mut conn = Connection::open(&test_db_path).unwrap();
        run_migrations(&mut conn).unwrap();

        create_session(&conn, "Deck1", 10).unwrap();
        create_session(&conn, "Deck2", 5).unwrap();
        create_session(&conn, "Deck3", 15).unwrap();

        let sessions = list_sessions(&conn).unwrap();
        assert_eq!(sessions.len(), 3);
        assert_eq!(sessions[0].deck_name, "Deck3"); // Newest first
        assert_eq!(sessions[1].deck_name, "Deck2");
        assert_eq!(sessions[2].deck_name, "Deck1");
    }

    #[test]
    fn test_get_session_detail() {
        use crate::db::flashcard::initialize_flashcards;

        let temp_dir = tempfile::tempdir().unwrap();
        let test_db_path = temp_dir.path().join("test.db");
        let mut conn = Connection::open(&test_db_path).unwrap();
        run_migrations(&mut conn).unwrap();

        let session_id = create_session(&conn, "Test Deck", 3).unwrap();
        let flashcards = vec![
            ("Q1".to_string(), "A1".to_string()),
            ("Q2".to_string(), "A2".to_string()),
        ];
        initialize_flashcards(&conn, session_id, &flashcards).unwrap();

        let detail = get_session_detail(&conn, session_id).unwrap().unwrap();
        assert_eq!(detail.0.deck_name, "Test Deck");
        assert_eq!(detail.1.len(), 2);
    }

    #[test]
    fn test_delete_session() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_db_path = temp_dir.path().join("test.db");
        let mut conn = Connection::open(&test_db_path).unwrap();
        run_migrations(&mut conn).unwrap();

        let session_id = create_session(&conn, "Test Deck", 10).unwrap();
        assert!(session_exists(&conn, session_id));

        delete_session(&conn, session_id).unwrap();
        assert!(!session_exists(&conn, session_id));

        let sessions = list_sessions(&conn).unwrap();
        assert_eq!(sessions.len(), 0);
    }
}
