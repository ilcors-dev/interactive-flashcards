use crate::ai::AIFeedback;
use rusqlite::{Connection, Result};
use serde_json;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct FlashcardData {
    pub id: u64,
    pub session_id: u64,
    pub created_at: u64,
    pub updated_at: u64,
    pub question: String,
    pub answer: String,
    pub user_answer: Option<String>,
    pub ai_feedback: Option<AIFeedback>,
    pub answered_at: Option<u64>,
    pub display_order: usize,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn initialize_flashcards(
    conn: &Connection,
    session_id: u64,
    flashcards: &[(String, String)],
) -> Result<Vec<u64>> {
    let created_at = now();
    let updated_at = created_at;
    let mut ids = Vec::new();

    for (index, (question, answer)) in flashcards.iter().enumerate() {
        conn.execute(
            "INSERT INTO flashcards (session_id, created_at, updated_at, question, answer, display_order)
             VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![session_id, created_at, updated_at, question, answer, index],
        )?;
        ids.push(conn.last_insert_rowid() as u64);
    }

    Ok(ids)
}

pub fn save_answer(
    conn: &Connection,
    session_id: u64,
    question: &str,
    _answer: &str,
    user_answer: &str,
    ai_feedback: Option<&AIFeedback>,
) -> Result<()> {
    let updated_at = now();
    let answered_at = updated_at;
    let ai_feedback_json = ai_feedback
        .map(|f| {
            serde_json::to_string(f)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))
        })
        .transpose()?;

    conn.execute(
        "UPDATE flashcards
         SET updated_at = ?, user_answer = ?, ai_feedback = ?, answered_at = ?
         WHERE session_id = ? AND question = ? AND display_order = (
             SELECT MIN(display_order)
             FROM flashcards
             WHERE session_id = ? AND user_answer IS NULL
         )",
        rusqlite::params![
            updated_at,
            user_answer,
            ai_feedback_json,
            answered_at,
            session_id,
            question,
            session_id
        ],
    )?;

    Ok(())
}

pub fn load_flashcards(conn: &Connection, session_id: u64) -> Result<Vec<FlashcardData>> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, created_at, updated_at, question, answer, user_answer, ai_feedback, answered_at, display_order
         FROM flashcards WHERE session_id = ? ORDER BY display_order",
    )?;

    let flashcards = stmt
        .query_map([session_id], |row| {
            let ai_feedback: Option<String> = row.get(7)?;
            let ai_feedback_parsed = ai_feedback
                .as_deref()
                .and_then(|f| serde_json::from_str::<AIFeedback>(f).ok());

            Ok(FlashcardData {
                id: row.get(0)?,
                session_id: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                question: row.get(4)?,
                answer: row.get(5)?,
                user_answer: row.get(6)?,
                ai_feedback: ai_feedback_parsed,
                answered_at: row.get(8)?,
                display_order: row.get(9)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(flashcards)
}

pub fn update_ai_feedback(
    conn: &Connection,
    flashcard_id: u64,
    ai_feedback: &AIFeedback,
) -> Result<()> {
    let updated_at = now();
    let ai_feedback_json = serde_json::to_string(ai_feedback)
        .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

    conn.execute(
        "UPDATE flashcards SET ai_feedback = ?, updated_at = ? WHERE id = ?",
        rusqlite::params![ai_feedback_json, updated_at, flashcard_id],
    )?;

    Ok(())
}

pub fn get_answer_count(conn: &Connection, session_id: u64) -> Result<usize> {
    let count: usize = conn.query_row(
        "SELECT COUNT(*) FROM flashcards WHERE session_id = ? AND user_answer IS NOT NULL",
        [session_id],
        |row| row.get(0),
    )?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::AIFeedback;
    use crate::db::{run_migrations, session::create_session};

    #[test]
    fn test_initialize_and_save_answer() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_db_path = temp_dir.path().join("test.db");
        let mut conn = Connection::open(&test_db_path).unwrap();
        run_migrations(&mut conn).unwrap();

        let session_id = create_session(&conn, "Test Deck", 3).unwrap();

        let flashcards = vec![
            ("Q1".to_string(), "A1".to_string()),
            ("Q2".to_string(), "A2".to_string()),
            ("Q3".to_string(), "A3".to_string()),
        ];
        let ids = initialize_flashcards(&conn, session_id, &flashcards).unwrap();
        assert_eq!(ids.len(), 3);

        save_answer(&conn, session_id, "Q1", "A1", "My Answer 1", None).unwrap();

        let loaded = load_flashcards(&conn, session_id).unwrap();
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].user_answer, Some("My Answer 1".to_string()));
        assert!(loaded[0].ai_feedback.is_none());
        assert!(loaded[1].user_answer.is_none());
    }

    #[test]
    fn test_save_answer_with_ai_feedback() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_db_path = temp_dir.path().join("test.db");
        let mut conn = Connection::open(&test_db_path).unwrap();
        run_migrations(&mut conn).unwrap();

        let session_id = create_session(&conn, "Test Deck", 1).unwrap();

        let flashcards = vec![("Q1".to_string(), "A1".to_string())];
        let _ = initialize_flashcards(&conn, session_id, &flashcards).unwrap();

        let ai_feedback = AIFeedback {
            is_correct: true,
            correctness_score: 1.0,
            corrections: vec![],
            explanation: "Correct!".to_string(),
            suggestions: vec![],
        };
        save_answer(
            &conn,
            session_id,
            "Q1",
            "A1",
            "My Answer",
            Some(&ai_feedback),
        )
        .unwrap();

        let loaded = load_flashcards(&conn, session_id).unwrap();
        assert!(loaded[0].ai_feedback.is_some());
        assert_eq!(loaded[0].ai_feedback.clone().unwrap().is_correct, true);
    }

    #[test]
    fn test_update_ai_feedback() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_db_path = temp_dir.path().join("test.db");
        let mut conn = Connection::open(&test_db_path).unwrap();
        run_migrations(&mut conn).unwrap();

        let session_id = create_session(&conn, "Test Deck", 1).unwrap();

        let flashcards = vec![("Q1".to_string(), "A1".to_string())];
        let _ = initialize_flashcards(&conn, session_id, &flashcards).unwrap();

        // Save initial answer without AI feedback
        save_answer(&conn, session_id, "Q1", "A1", "My Answer", None).unwrap();

        let loaded = load_flashcards(&conn, session_id).unwrap();
        let flashcard_id = loaded[0].id;
        assert!(loaded[0].ai_feedback.is_none());

        // Update with AI feedback
        let ai_feedback = AIFeedback {
            is_correct: true,
            correctness_score: 0.85,
            corrections: vec!["Minor correction".to_string()],
            explanation: "Good answer!".to_string(),
            suggestions: vec!["Keep it up!".to_string()],
        };

        update_ai_feedback(&conn, flashcard_id, &ai_feedback).unwrap();

        let loaded_after_update = load_flashcards(&conn, session_id).unwrap();
        assert!(loaded_after_update[0].ai_feedback.is_some());

        let saved_feedback = loaded_after_update[0].ai_feedback.as_ref().unwrap();
        assert_eq!(saved_feedback.is_correct, true);
        assert_eq!(saved_feedback.correctness_score, 0.85);
        assert_eq!(
            saved_feedback.corrections,
            vec!["Minor correction".to_string()]
        );
        assert_eq!(saved_feedback.explanation, "Good answer!");
        assert_eq!(saved_feedback.suggestions, vec!["Keep it up!".to_string()]);
    }

    #[test]
    fn test_update_ai_feedback_invalid_id() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_db_path = temp_dir.path().join("test.db");
        let mut conn = Connection::open(&test_db_path).unwrap();
        run_migrations(&mut conn).unwrap();

        let ai_feedback = AIFeedback {
            is_correct: true,
            correctness_score: 1.0,
            corrections: vec![],
            explanation: "Test".to_string(),
            suggestions: vec![],
        };

        // Should not panic or find any record to update
        let result = update_ai_feedback(&conn, 999, &ai_feedback);
        assert!(result.is_ok()); // SQLite UPDATE with no matching rows returns Ok
    }

    #[test]
    fn test_complete_ai_feedback_cycle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_db_path = temp_dir.path().join("test.db");
        let mut conn = Connection::open(&test_db_path).unwrap();
        run_migrations(&mut conn).unwrap();

        let session_id = create_session(&conn, "Test Deck", 2).unwrap();

        // Initialize 2 flashcards
        let flashcards = vec![
            ("Question 1".to_string(), "Answer 1".to_string()),
            ("Question 2".to_string(), "Answer 2".to_string()),
        ];
        let _ = initialize_flashcards(&conn, session_id, &flashcards).unwrap();

        // Simulate answering first question without AI feedback initially
        save_answer(
            &conn,
            session_id,
            "Question 1",
            "Answer 1",
            "User Answer 1",
            None,
        )
        .unwrap();

        let loaded_after_answer = load_flashcards(&conn, session_id).unwrap();
        assert_eq!(
            loaded_after_answer[0].user_answer,
            Some("User Answer 1".to_string())
        );
        assert!(loaded_after_answer[0].ai_feedback.is_none());
        assert!(loaded_after_answer[1].user_answer.is_none());

        // Simulate AI evaluation completing and updating the flashcard
        let ai_feedback = AIFeedback {
            is_correct: false,
            correctness_score: 0.60,
            corrections: vec!["Better answer would be...".to_string()],
            explanation: "Partially correct".to_string(),
            suggestions: vec!["Study more".to_string()],
        };

        let flashcard_id = loaded_after_answer[0].id;
        update_ai_feedback(&conn, flashcard_id, &ai_feedback).unwrap();

        // Load again and verify AI feedback is now present
        let loaded_after_ai = load_flashcards(&conn, session_id).unwrap();

        // First flashcard should have both answer and AI feedback
        assert_eq!(
            loaded_after_ai[0].user_answer,
            Some("User Answer 1".to_string())
        );
        assert!(loaded_after_ai[0].ai_feedback.is_some());

        let saved_feedback = loaded_after_ai[0].ai_feedback.as_ref().unwrap();
        assert_eq!(saved_feedback.is_correct, false);
        assert_eq!(saved_feedback.correctness_score, 0.60);
        assert_eq!(
            saved_feedback.corrections,
            vec!["Better answer would be...".to_string()]
        );
        assert_eq!(saved_feedback.explanation, "Partially correct");
        assert_eq!(saved_feedback.suggestions, vec!["Study more"]);

        // Second flashcard should remain unanswered
        assert!(loaded_after_ai[1].user_answer.is_none());
        assert!(loaded_after_ai[1].ai_feedback.is_none());

        // Answer second question with AI feedback from the start
        let ai_feedback_2 = AIFeedback {
            is_correct: true,
            correctness_score: 1.0,
            corrections: vec![],
            explanation: "Perfect!".to_string(),
            suggestions: vec!["Great work".to_string()],
        };

        save_answer(
            &conn,
            session_id,
            "Question 2",
            "Answer 2",
            "User Answer 2",
            Some(&ai_feedback_2),
        )
        .unwrap();

        // Final verification
        let final_loaded = load_flashcards(&conn, session_id).unwrap();
        assert_eq!(final_loaded.len(), 2);

        // First flashcard: answer + later AI update
        assert_eq!(
            final_loaded[0].user_answer,
            Some("User Answer 1".to_string())
        );
        assert!(final_loaded[0].ai_feedback.is_some());
        assert_eq!(
            final_loaded[0]
                .ai_feedback
                .as_ref()
                .unwrap()
                .correctness_score,
            0.60
        );

        // Second flashcard: answer + immediate AI feedback
        assert_eq!(
            final_loaded[1].user_answer,
            Some("User Answer 2".to_string())
        );
        assert!(final_loaded[1].ai_feedback.is_some());
        assert_eq!(
            final_loaded[1]
                .ai_feedback
                .as_ref()
                .unwrap()
                .correctness_score,
            1.0
        );
    }

    #[test]
    fn test_get_answer_count() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_db_path = temp_dir.path().join("test.db");
        let mut conn = Connection::open(&test_db_path).unwrap();
        run_migrations(&mut conn).unwrap();

        let session_id = create_session(&conn, "Test Deck", 3).unwrap();

        let flashcards = vec![
            ("Q1".to_string(), "A1".to_string()),
            ("Q2".to_string(), "A2".to_string()),
            ("Q3".to_string(), "A3".to_string()),
        ];
        let _ = initialize_flashcards(&conn, session_id, &flashcards).unwrap();

        assert_eq!(get_answer_count(&conn, session_id).unwrap(), 0);

        save_answer(&conn, session_id, "Q1", "A1", "A1", None).unwrap();
        assert_eq!(get_answer_count(&conn, session_id).unwrap(), 1);

        save_answer(&conn, session_id, "Q2", "A2", "A2", None).unwrap();
        assert_eq!(get_answer_count(&conn, session_id).unwrap(), 2);
    }
}
