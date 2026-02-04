use rusqlite::{Connection, Result};
use std::path::PathBuf;

pub mod chat;
pub mod flashcard;
pub mod session;

mod embedded_migrations {
    use refinery::embed_migrations;
    embed_migrations!("./src/db/migrations");
}

fn get_data_dir() -> PathBuf {
    if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
        PathBuf::from(home).join(".local/share/interactive-flashcards")
    } else if cfg!(target_os = "windows") {
        let home = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\User".to_string());
        PathBuf::from(home).join(".local\\share\\interactive-flashcards")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
        PathBuf::from(home).join(".local/share/interactive-flashcards")
    }
}

pub fn get_db_path() -> PathBuf {
    get_data_dir().join("if.db")
}

pub fn init_db() -> Result<Connection> {
    let db_path = get_db_path();

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let mut conn = Connection::open(&db_path)?;

    run_migrations(&mut conn)?;

    Ok(conn)
}

fn run_migrations(conn: &mut Connection) -> Result<()> {
    embedded_migrations::migrations::runner()
        .run(conn)
        .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
pub fn run_migrations_for_test(conn: &mut Connection) -> Result<()> {
    run_migrations(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_db_creates_directory() {
        let test_db_path = std::env::temp_dir().join("test_if.db");
        let mut conn = Connection::open(&test_db_path).unwrap();
        run_migrations(&mut conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"sessions".to_string()));
        assert!(tables.contains(&"flashcards".to_string()));
    }

    #[test]
    fn test_create_session() {
        use super::session::create_session;

        let temp_dir = tempfile::tempdir().unwrap();
        let test_db_path = temp_dir.path().join("test.db");
        let mut conn = Connection::open(&test_db_path).unwrap();
        run_migrations(&mut conn).unwrap();

        let session_id = create_session(&conn, "Test Deck", 10).unwrap();
        assert_eq!(session_id, 1);

        let session = session::get_session(&conn, session_id).unwrap();
        assert!(session.is_some());
        let s = session.unwrap();
        assert_eq!(s.deck_name, "Test Deck");
        assert_eq!(s.questions_total, 10);
        assert_eq!(s.questions_answered, 0);
        assert!(s.completed_at.is_none());
    }

    #[test]
    fn test_update_progress() {
        use super::session::{create_session, update_progress};

        let temp_dir = tempfile::tempdir().unwrap();
        let test_db_path = temp_dir.path().join("test.db");
        let mut conn = Connection::open(&test_db_path).unwrap();
        run_migrations(&mut conn).unwrap();

        let session_id = create_session(&conn, "Test Deck", 10).unwrap();
        update_progress(&conn, session_id, 5, 0.0).unwrap();

        let session = session::get_session(&conn, session_id).unwrap().unwrap();
        assert_eq!(session.questions_answered, 5);
    }

    #[test]
    fn test_complete_session() {
        use super::session::{complete_session, create_session, get_session};

        let temp_dir = tempfile::tempdir().unwrap();
        let test_db_path = temp_dir.path().join("test.db");
        let mut conn = Connection::open(&test_db_path).unwrap();
        run_migrations(&mut conn).unwrap();

        let session_id = create_session(&conn, "Test Deck", 10).unwrap();
        complete_session(&conn, session_id).unwrap();

        let session = get_session(&conn, session_id).unwrap().unwrap();
        assert!(session.completed_at.is_some());
    }
}
