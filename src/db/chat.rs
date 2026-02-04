use crate::models::{ChatMessage, ChatRole};
use rusqlite::{Connection, Result};
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn save_chat_message(
    conn: &Connection,
    flashcard_id: u64,
    session_id: u64,
    role: &ChatRole,
    content: &str,
    order: u32,
) -> Result<u64> {
    let ts = now();
    conn.execute(
        "INSERT INTO chat_messages (flashcard_id, session_id, role, content, message_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![flashcard_id, session_id, role.as_str(), content, order, ts, ts],
    )?;
    Ok(conn.last_insert_rowid() as u64)
}

pub fn load_chat_messages(conn: &Connection, flashcard_id: u64) -> Result<Vec<ChatMessage>> {
    let mut stmt = conn.prepare(
        "SELECT id, role, content, message_order FROM chat_messages
         WHERE flashcard_id = ? ORDER BY message_order ASC",
    )?;

    let messages = stmt
        .query_map([flashcard_id], |row| {
            Ok(ChatMessage {
                id: Some(row.get::<_, u64>(0)?),
                role: ChatRole::parse(&row.get::<_, String>(1)?),
                content: row.get(2)?,
                message_order: row.get(3)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::run_migrations_for_test;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        run_migrations_for_test(&mut conn).unwrap();
        conn
    }

    #[test]
    fn test_save_and_load_chat_messages() {
        let conn = setup_db();

        let session_id = crate::db::session::create_session(&conn, "Test", 1).unwrap();
        let flashcards = vec![("Q1".to_string(), "A1".to_string())];
        let ids =
            crate::db::flashcard::initialize_flashcards(&conn, session_id, &flashcards).unwrap();
        let flashcard_id = ids[0];

        save_chat_message(&conn, flashcard_id, session_id, &ChatRole::User, "Hello", 0).unwrap();
        save_chat_message(
            &conn,
            flashcard_id,
            session_id,
            &ChatRole::Assistant,
            "Hi there",
            1,
        )
        .unwrap();

        let messages = load_chat_messages(&conn, flashcard_id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, ChatRole::User);
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].role, ChatRole::Assistant);
        assert_eq!(messages[1].content, "Hi there");
    }

    #[test]
    fn test_load_empty_chat() {
        let conn = setup_db();
        let messages = load_chat_messages(&conn, 999).unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn test_message_ordering() {
        let conn = setup_db();

        let session_id = crate::db::session::create_session(&conn, "Test", 1).unwrap();
        let flashcards = vec![("Q1".to_string(), "A1".to_string())];
        let ids =
            crate::db::flashcard::initialize_flashcards(&conn, session_id, &flashcards).unwrap();
        let flashcard_id = ids[0];

        save_chat_message(&conn, flashcard_id, session_id, &ChatRole::User, "First", 0).unwrap();
        save_chat_message(
            &conn,
            flashcard_id,
            session_id,
            &ChatRole::Assistant,
            "Second",
            1,
        )
        .unwrap();
        save_chat_message(&conn, flashcard_id, session_id, &ChatRole::User, "Third", 2).unwrap();

        let messages = load_chat_messages(&conn, flashcard_id).unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].message_order, 0);
        assert_eq!(messages[1].message_order, 1);
        assert_eq!(messages[2].message_order, 2);
    }

    #[test]
    fn test_chat_role_roundtrip() {
        assert_eq!(ChatRole::parse(ChatRole::User.as_str()), ChatRole::User);
        assert_eq!(
            ChatRole::parse(ChatRole::Assistant.as_str()),
            ChatRole::Assistant
        );
        assert_eq!(ChatRole::parse(ChatRole::System.as_str()), ChatRole::System);
    }
}
