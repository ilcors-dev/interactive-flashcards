use crate::db::{self, flashcard, session};
use crate::logger;
use crate::models::{AiRequest, AiResponse, AppState, QuizSession};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::io;

pub fn handle_quiz_input(
    session: &mut QuizSession,
    key: KeyEvent,
    app_state: &mut AppState,
) -> io::Result<()> {
    if !session.showing_answer {
        match key.code {
            KeyCode::Esc => {
                *app_state = AppState::QuizQuitConfirm;
                Ok(())
            }
            KeyCode::Down => {
                if session.current_index < session.flashcards.len().saturating_sub(1) {
                    session.current_index += 1;
                    // Show answer screen if question was already answered, otherwise show input
                    session.showing_answer = session.flashcards[session.current_index]
                        .user_answer
                        .is_some();
                    session.last_ai_error = None;
                    if !session.showing_answer {
                        // Restore input buffer for unanswered questions
                        session.input_buffer = session.flashcards[session.current_index]
                            .user_answer
                            .as_ref()
                            .unwrap_or(&String::new())
                            .clone();
                        session.cursor_position = session.input_buffer.len();
                        session.input_scroll_y = 0; // Reset scroll on question navigation
                    }
                }
                Ok(())
            }
            KeyCode::Up => {
                if session.current_index > 0 {
                    session.current_index -= 1;
                    // Show answer screen if question was already answered, otherwise show input
                    session.showing_answer = session.flashcards[session.current_index]
                        .user_answer
                        .is_some();
                    session.last_ai_error = None;
                    if !session.showing_answer {
                        // Restore input buffer for unanswered questions
                        session.input_buffer = session.flashcards[session.current_index]
                            .user_answer
                            .as_ref()
                            .unwrap_or(&String::new())
                            .clone();
                        session.cursor_position = session.input_buffer.len();
                        session.input_scroll_y = 0; // Reset scroll on question navigation
                    }
                }
                Ok(())
            }
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    session.input_buffer.insert(session.cursor_position, '\n');
                    session.cursor_position += 1;
                    Ok(())
                } else if !session.input_buffer.trim().is_empty() {
                    session.flashcards[session.current_index].user_answer =
                        Some(session.input_buffer.clone());
                    session.flashcards[session.current_index].written_to_file = false;

                    session.questions_answered += 1;

                    if let Some(session_id) = session.session_id {
                        let conn = match db::init_db() {
                            Ok(conn) => conn,
                            Err(e) => {
                                return Err(io::Error::other(format!("DB error: {}", e)));
                            }
                        };

                        let current_card = &session.flashcards[session.current_index];
                        let user_answer = current_card.user_answer.as_deref().unwrap_or("");
                        let ai_feedback = current_card.ai_feedback.as_ref();

                        if let Err(e) = flashcard::save_answer(
                            &conn,
                            session_id,
                            &current_card.question,
                            &current_card.answer,
                            user_answer,
                            ai_feedback,
                        ) {
                            return Err(io::Error::other(format!("DB error: {}", e)));
                        }
                        session.flashcards[session.current_index].written_to_file = true;

                        if let Err(e) =
                            session::update_progress(&conn, session_id, session.questions_answered)
                        {
                            return Err(io::Error::other(format!("DB error: {}", e)));
                        }
                    }

                    session.last_ai_error = None;
                    session.input_buffer.clear();
                    session.cursor_position = 0;
                    session.showing_answer = true;

                    if session.ai_enabled {
                        session.request_ai_evaluation(session.current_index);
                    }

                    Ok(())
                } else {
                    Ok(())
                }
            }
            KeyCode::Left => {
                if session.cursor_position > 0 {
                    session.cursor_position -= 1;
                }
                // Ensure cursor doesn't go beyond buffer bounds
                session.cursor_position = session.cursor_position.min(session.input_buffer.len());
                Ok(())
            }
            KeyCode::Right => {
                if session.cursor_position < session.input_buffer.len() {
                    session.cursor_position += 1;
                }
                Ok(())
            }
            KeyCode::Backspace => {
                if session.cursor_position > 0 {
                    session.input_buffer.remove(session.cursor_position - 1);
                    session.cursor_position -= 1;
                }
                Ok(())
            }
            KeyCode::Char(c) => {
                session.input_buffer.insert(session.cursor_position, c);
                session.cursor_position += 1;
                Ok(())
            }
            _ => Ok(()),
        }
    } else {
        match key.code {
            KeyCode::Esc => {
                *app_state = AppState::QuizQuitConfirm;
                Ok(())
            }
            KeyCode::Down => {
                if session.current_index < session.flashcards.len().saturating_sub(1) {
                    session.current_index += 1;
                    // Show answer screen if question was already answered, otherwise show input
                    session.showing_answer = session.flashcards[session.current_index]
                        .user_answer
                        .is_some();
                    session.last_ai_error = None;
                    if !session.showing_answer {
                        // Restore input buffer for unanswered questions
                        session.input_buffer = session.flashcards[session.current_index]
                            .user_answer
                            .as_ref()
                            .unwrap_or(&String::new())
                            .clone();
                        session.cursor_position = session.input_buffer.len();
                        session.input_scroll_y = 0; // Reset scroll on question navigation
                    }
                }
                Ok(())
            }
            KeyCode::Up => {
                if session.current_index > 0 {
                    session.current_index -= 1;
                    // Show answer screen if question was already answered, otherwise show input
                    session.showing_answer = session.flashcards[session.current_index]
                        .user_answer
                        .is_some();
                    session.last_ai_error = None;
                    if !session.showing_answer {
                        // Restore input buffer for unanswered questions
                        session.input_buffer = session.flashcards[session.current_index]
                            .user_answer
                            .as_ref()
                            .unwrap_or(&String::new())
                            .clone();
                        session.cursor_position = session.input_buffer.len();
                        session.input_scroll_y = 0; // Reset scroll on question navigation
                    }
                }
                Ok(())
            }
            KeyCode::Enter => {
                if session.current_index < session.flashcards.len().saturating_sub(1) {
                    session.current_index += 1;
                    // Show answer screen if question was already answered, otherwise show input
                    session.showing_answer = session.flashcards[session.current_index]
                        .user_answer
                        .is_some();
                    session.last_ai_error = None;
                    if !session.showing_answer {
                        // Restore input buffer for unanswered questions
                        session.input_buffer = session.flashcards[session.current_index]
                            .user_answer
                            .as_ref()
                            .unwrap_or(&String::new())
                            .clone();
                        session.cursor_position = session.input_buffer.len();
                        session.input_scroll_y = 0; // Reset scroll on question navigation
                    }
                } else {
                    *app_state = AppState::Summary;
                    session.assessment_loading = true;
                    session.assessment_error = None;
                }
                Ok(())
            }
            KeyCode::Char('e') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && session.ai_enabled {
                    session.last_ai_error = None;
                    session.manual_trigger_ai_evaluation();
                }
                Ok(())
            }
            KeyCode::Char('x') => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && session.ai_enabled
                    && session.ai_evaluation_in_progress
                {
                    session.ai_evaluation_in_progress = false;
                    session.last_ai_error = Some("Evaluation cancelled".to_string());
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

impl QuizSession {
    pub fn request_ai_evaluation(&mut self, flashcard_index: usize) {
        if !self.ai_enabled || self.ai_evaluation_in_progress {
            return;
        }

        if let Some(last_idx) = self.ai_last_evaluated_index
            && last_idx == flashcard_index {
                return;
            }

        let flashcard = &self.flashcards[flashcard_index];
        let user_answer = match &flashcard.user_answer {
            Some(ans) => ans.clone(),
            None => return,
        };

        if user_answer.trim().is_empty() {
            return;
        }

        self.last_ai_error = None; // Clear any previous error before starting new evaluation
        self.ai_evaluation_start_time = Some(std::time::Instant::now()); // Track when evaluation started
        logger::log(&format!(
            "Sending AI request for flashcard {}",
            flashcard_index
        ));

        if let Some(ai_tx) = self.ai_tx.clone() {
            let request = AiRequest::Evaluate {
                flashcard_index,
                question: flashcard.question.clone(),
                correct_answer: flashcard.answer.clone(),
                user_answer: user_answer.clone(),
            };
            tokio::spawn(async move {
                let _ = ai_tx.send(request).await;
            });
            logger::log("AI request sent through async channel");
        }

        self.ai_evaluation_in_progress = true;
        logger::log("Set ai_evaluation_in_progress = true");
    }

    pub fn manual_trigger_ai_evaluation(&mut self) {
        self.ai_evaluation_in_progress = false;
        if self.ai_enabled {
            self.request_ai_evaluation(self.current_index);
        }
    }

    pub fn process_ai_responses(&mut self, response: AiResponse) {
        let (flashcard_index, feedback) = match response {
            AiResponse::Evaluation {
                flashcard_index,
                result,
            } => {
                logger::log(&format!(
                    "Received evaluation for flashcard {}: score {:.2}",
                    flashcard_index, result.feedback.correctness_score
                ));
                self.ai_last_evaluated_index = Some(flashcard_index);
                self.ai_evaluation_in_progress = false;
                self.last_ai_error = None; // Clear any previous error so feedback can display
                logger::log("Set ai_evaluation_in_progress = false (success)");
                (flashcard_index, Some(result.feedback))
            }
            AiResponse::Error {
                flashcard_index,
                error,
            } => {
                logger::log(&format!(
                    "Received error for flashcard {}: {}",
                    flashcard_index, error
                ));
                self.ai_evaluation_in_progress = false;
                self.last_ai_error = Some(error.clone());
                logger::log("Set ai_evaluation_in_progress = false (error)");
                (
                    flashcard_index,
                    Some(crate::ai::AIFeedback {
                        is_correct: false,
                        correctness_score: 0.0,
                        corrections: vec![],
                        explanation: format!("Error: {}", error),
                        suggestions: vec![],
                    }),
                )
            }
            AiResponse::SessionAssessment {
                session_id: _,
                result,
            } => {
                logger::log("Received session assessment response");
                self.assessment_loading = false;
                match result {
                    Ok(assessment) => {
                        self.session_assessment = Some(assessment);
                        self.assessment_error = None;
                        logger::log("Session assessment loaded successfully");
                    }
                    Err(error) => {
                        self.session_assessment = None;
                        self.assessment_error = Some(error.clone());
                        logger::log(&format!("Session assessment error: {}", error));
                    }
                }
                return; // Session assessment doesn't update flashcard feedback
            }
        };
        self.flashcards[flashcard_index].ai_feedback = feedback;

        if let Some(session_id) = self.session_id
            && let Ok(ref conn) = db::init_db() {
            if let Some(flashcard_id) = self.flashcards[flashcard_index].id {
                    if let Some(ai_feedback) = &self.flashcards[flashcard_index].ai_feedback {
                        crate::db::flashcard::update_ai_feedback(conn, flashcard_id, ai_feedback)
                            .unwrap_or_else(|e| {
                                crate::logger::log(&format!(
                                    "Failed to update AI feedback for flashcard {}: {}",
                                    flashcard_id, e
                                ));
                            });
                    }
                } else if !self.flashcards[flashcard_index].written_to_file {
                    // New flashcard - save answer with AI feedback
                    let current_card = &self.flashcards[flashcard_index];
                    let user_answer = current_card.user_answer.as_deref().unwrap_or("");
                    let ai_feedback = current_card.ai_feedback.as_ref();

                    flashcard::save_answer(
                        conn,
                        session_id,
                        &current_card.question,
                        &current_card.answer,
                        user_answer,
                        ai_feedback,
                    ).ok();
                    self.flashcards[flashcard_index].written_to_file = true;
                }
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AppState, Flashcard, QuizSession};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    #[test]
    fn test_input_buffer_operations() {
        let mut buffer = String::new();
        buffer.push('H');
        buffer.push('i');
        assert_eq!(buffer, "Hi");
        buffer.pop();
        assert_eq!(buffer, "H");
        assert!(buffer.trim().is_empty() == false);
    }

    #[test]
    fn test_empty_answer_submission() {
        let mut buffer = String::new();
        assert!(buffer.trim().is_empty());
        buffer.push(' ');
        assert!(buffer.trim().is_empty());
        buffer.push('A');
        assert!(!buffer.trim().is_empty());
    }

    #[test]
    fn test_saturating_sub_index_bounds() {
        let cards_len: usize = 1;
        let current_index: usize = 0;
        let new_index = current_index.saturating_sub(1);
        assert_eq!(new_index, 0);

        let max_index = cards_len.saturating_sub(1);
        assert_eq!(max_index, 0);
    }

    #[test]
    fn test_answer_restoration_on_navigation() {
        let user_answer = Some("My Answer 1".to_string());
        let input_buffer = user_answer.as_ref().unwrap_or(&String::new()).clone();

        assert_eq!(input_buffer, "My Answer 1");
    }

    #[test]
    fn test_no_answer_restoration_when_none() {
        let user_answer: Option<String> = None;
        let input_buffer = user_answer.as_ref().unwrap_or(&String::new()).clone();

        assert!(input_buffer.is_empty());
    }

    #[test]
    fn test_answer_submission_non_empty() {
        let input_buffer = String::from("My Answer");
        let mut user_answer: Option<String> = None;

        if !input_buffer.trim().is_empty() {
            user_answer = Some(input_buffer.clone());
        }

        assert_eq!(user_answer, Some("My Answer".to_string()));
    }

    #[test]
    fn test_answer_submission_empty() {
        let input_buffer = String::from("   ");
        let mut user_answer: Option<String> = None;

        if !input_buffer.trim().is_empty() {
            user_answer = Some(input_buffer.clone());
        }

        assert!(user_answer.is_none());
    }

    #[test]
    fn test_input_buffer_backspace_basic() {
        let mut buffer = String::from("Hello");
        buffer.pop();
        assert_eq!(buffer, "Hell");
        buffer.pop();
        assert_eq!(buffer, "Hel");
        buffer.pop();
        assert_eq!(buffer, "He");
        buffer.pop();
        assert_eq!(buffer, "H");
        buffer.pop();
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_input_buffer_character_addition() {
        let mut buffer = String::new();
        buffer.push('H');
        buffer.push('e');
        buffer.push('l');
        buffer.push('l');
        buffer.push('o');
        assert_eq!(buffer, "Hello");
        buffer.push(' ');
        buffer.push('W');
        buffer.push('o');
        buffer.push('r');
        buffer.push('l');
        buffer.push('d');
        assert_eq!(buffer, "Hello World");
    }

    #[test]
    fn test_input_buffer_backspace() {
        let mut buffer = String::from("Hello");
        buffer.pop();
        assert_eq!(buffer, "Hell");
        buffer.pop();
        buffer.pop();
        assert_eq!(buffer, "He");
        buffer.pop();
        buffer.pop();
        buffer.pop();
        assert!(buffer.is_empty());
        buffer.pop();
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_can_type_r_and_c_in_answers() {
        use tokio::sync::mpsc;

        let (tx, _rx) = mpsc::channel(32);
        let mut session = QuizSession {
            flashcards: vec![Flashcard {
                question: "Test?".to_string(),
                answer: "Answer".to_string(),
                user_answer: None,
                ai_feedback: None,
                written_to_file: false,
                id: None,
            }],
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
            cursor_position: 0,
            session_id: None,
            questions_total: 1,
            questions_answered: 0,
            ai_enabled: false,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: Some(tx),
            ai_rx: None,
            input_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
        };
        let app_state = &mut AppState::Quiz;

        // Test typing 'r'
        let r_key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, r_key, app_state);
        assert_eq!(session.input_buffer, "r");

        // Test typing 'c'
        let c_key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, c_key, app_state);
        assert_eq!(session.input_buffer, "rc");

        // Test typing 'R' and 'C'
        let r_upper = KeyEvent::new(KeyCode::Char('R'), KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, r_upper, app_state);
        assert_eq!(session.input_buffer, "rcR");

        let c_upper = KeyEvent::new(KeyCode::Char('C'), KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, c_upper, app_state);
        assert_eq!(session.input_buffer, "rcRC");
    }

    #[tokio::test]
    async fn test_ctrl_e_triggers_ai_evaluation() {
        use tokio::sync::mpsc;

        let (tx, _rx) = mpsc::channel(32);
        let mut session = QuizSession {
            flashcards: vec![Flashcard {
                question: "Test?".to_string(),
                answer: "Answer".to_string(),
                user_answer: Some("test answer".to_string()),
                ai_feedback: None,
                written_to_file: false,
                id: None,
            }],
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: true, // Need to be showing answer for AI commands
            input_buffer: String::new(),
            cursor_position: 0,
            session_id: None,
            questions_total: 1,
            questions_answered: 1,
            ai_enabled: true,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: Some(tx),
            ai_rx: None,

            input_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
        };
        let app_state = &mut AppState::Quiz;

        let ctrl_e = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
        let _ = handle_quiz_input(&mut session, ctrl_e, app_state);

        // Should trigger evaluation and clear errors
        assert!(session.ai_evaluation_in_progress);
        assert!(session.last_ai_error.is_none());
    }

    #[test]
    fn test_ctrl_x_cancels_ai_evaluation() {
        let mut session = QuizSession {
            flashcards: vec![Flashcard {
                question: "Test?".to_string(),
                answer: "Answer".to_string(),
                user_answer: Some("test answer".to_string()),
                ai_feedback: None,
                written_to_file: false,
                id: None,
            }],
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: true,
            input_buffer: String::new(),
            cursor_position: 0,
            session_id: None,
            questions_total: 1,
            questions_answered: 1,
            ai_enabled: true,
            ai_evaluation_in_progress: true,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: None,
            ai_rx: None,

            input_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
        };
        let app_state = &mut AppState::Quiz;

        let ctrl_x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
        let _ = handle_quiz_input(&mut session, ctrl_x, app_state);

        // Should cancel evaluation and show message
        assert!(!session.ai_evaluation_in_progress);
        assert_eq!(
            session.last_ai_error,
            Some("Evaluation cancelled".to_string())
        );
    }

    #[test]
    fn test_ctrl_e_x_without_ctrl_modifier_allows_typing() {
        let mut session = QuizSession {
            flashcards: vec![Flashcard {
                question: "Test?".to_string(),
                answer: "Answer".to_string(),
                user_answer: None,
                ai_feedback: None,
                written_to_file: false,
                id: None,
            }],
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false, // Need to be in input mode for typing
            input_buffer: String::new(),
            cursor_position: 0,
            session_id: None,
            questions_total: 1,
            questions_answered: 0,
            ai_enabled: true,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: None,
            ai_rx: None,

            input_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
        };
        let app_state = &mut AppState::Quiz;

        // Typing 'e' without Ctrl should add to buffer
        let e_key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, e_key, app_state);
        assert_eq!(session.input_buffer, "e");

        // Typing 'x' without Ctrl should add to buffer
        let x_key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, x_key, app_state);
        assert_eq!(session.input_buffer, "ex");
    }

    #[test]
    fn test_ai_commands_only_work_when_enabled() {
        let mut session = QuizSession {
            flashcards: vec![Flashcard {
                question: "Test?".to_string(),
                answer: "Answer".to_string(),
                user_answer: Some("test answer".to_string()),
                ai_feedback: None,
                written_to_file: false,
                id: None,
            }],
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: true,
            input_buffer: String::new(),
            cursor_position: 0,
            session_id: None,
            questions_total: 1,
            questions_answered: 1,
            ai_enabled: false, // AI disabled
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: None,
            ai_rx: None,

            input_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
        };
        let app_state = &mut AppState::Quiz;

        let ctrl_e = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
        let _ = handle_quiz_input(&mut session, ctrl_e, app_state);

        // Should not trigger evaluation when AI disabled
        assert!(!session.ai_evaluation_in_progress);
    }

    #[test]
    fn test_ctrl_x_only_works_during_evaluation() {
        let mut session = QuizSession {
            flashcards: vec![Flashcard {
                question: "Test?".to_string(),
                answer: "Answer".to_string(),
                user_answer: Some("test answer".to_string()),
                ai_feedback: None,
                written_to_file: false,
                id: None,
            }],
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: true,
            input_buffer: String::new(),
            cursor_position: 0,
            session_id: None,
            questions_total: 1,
            questions_answered: 1,
            ai_enabled: true,
            ai_evaluation_in_progress: false, // No evaluation in progress
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: None,
            ai_rx: None,

            input_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
        };
        let app_state = &mut AppState::Quiz;

        let ctrl_x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
        let _ = handle_quiz_input(&mut session, ctrl_x, app_state);

        // Should not do anything when no evaluation is in progress
        assert!(!session.ai_evaluation_in_progress);
        assert!(session.last_ai_error.is_none());
    }

    #[test]
    fn test_cursor_left_right_movement() {
        use tokio::sync::mpsc;

        let (tx, _rx) = mpsc::channel(32);
        let mut session = QuizSession {
            flashcards: vec![Flashcard {
                question: "Test?".to_string(),
                answer: "Answer".to_string(),
                user_answer: None,
                ai_feedback: None,
                written_to_file: false,
                id: None,
            }],
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false, // Need to be in input mode
            input_buffer: "Hello".to_string(),
            cursor_position: 5, // Start at end of "Hello"
            session_id: None,
            questions_total: 1,
            questions_answered: 0,
            ai_enabled: false,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: Some(tx),
            ai_rx: None,

            input_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
        };
        let app_state = &mut AppState::Quiz;

        // Test moving cursor left
        let left_key = KeyEvent::new(KeyCode::Left, KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, left_key, app_state);
        assert_eq!(session.cursor_position, 4);

        let _ = handle_quiz_input(&mut session, left_key, app_state);
        assert_eq!(session.cursor_position, 3);

        // Test moving cursor right
        let right_key = KeyEvent::new(KeyCode::Right, KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, right_key, app_state);
        assert_eq!(session.cursor_position, 4);

        // Test bounds: can't go left of position 0
        for _ in 0..10 {
            let _ = handle_quiz_input(&mut session, left_key, app_state);
        }
        assert_eq!(session.cursor_position, 0);

        // Test bounds: can't go right past string length
        for _ in 0..10 {
            let _ = handle_quiz_input(&mut session, right_key, app_state);
        }
        assert_eq!(session.cursor_position, 5); // Length of "Hello"
    }

    #[test]
    fn test_insert_character_at_cursor_position() {
        use tokio::sync::mpsc;

        let (tx, _rx) = mpsc::channel(32);
        let mut session = QuizSession {
            flashcards: vec![Flashcard {
                question: "Test?".to_string(),
                answer: "Answer".to_string(),
                user_answer: None,
                ai_feedback: None,
                written_to_file: false,
                id: None,
            }],
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: "Helo".to_string(),
            cursor_position: 3, // Between 'e' and 'o'
            session_id: None,
            questions_total: 1,
            questions_answered: 0,
            ai_enabled: false,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: Some(tx),
            ai_rx: None,

            input_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
        };
        let app_state = &mut AppState::Quiz;

        // Insert 'l' at position 3 (between 'e' and 'o')
        let l_key = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, l_key, app_state);

        assert_eq!(session.input_buffer, "Hello");
        assert_eq!(session.cursor_position, 4); // Cursor should advance

        // Move cursor to beginning and insert
        session.cursor_position = 0;
        let w_key = KeyEvent::new(KeyCode::Char('W'), KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, w_key, app_state);

        assert_eq!(session.input_buffer, "WHello");
        assert_eq!(session.cursor_position, 1);
    }

    #[test]
    fn test_backspace_deletes_at_cursor_position() {
        use tokio::sync::mpsc;

        let (tx, _rx) = mpsc::channel(32);
        let mut session = QuizSession {
            flashcards: vec![Flashcard {
                question: "Test?".to_string(),
                answer: "Answer".to_string(),
                user_answer: None,
                ai_feedback: None,
                written_to_file: false,
                id: None,
            }],
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: "Hello World".to_string(),
            cursor_position: 5, // At space between "Hello" and "World"
            session_id: None,
            questions_total: 1,
            questions_answered: 0,
            ai_enabled: false,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: Some(tx),
            ai_rx: None,

            input_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
        };
        let app_state = &mut AppState::Quiz;

        // Backspace should delete the character before cursor ('o')
        let backspace_key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, backspace_key, app_state);

        assert_eq!(session.input_buffer, "Hell World");
        assert_eq!(session.cursor_position, 4); // Cursor should move left

        // Move cursor to end and backspace
        session.cursor_position = session.input_buffer.len();
        let _ = handle_quiz_input(&mut session, backspace_key, app_state);

        assert_eq!(session.input_buffer, "Hell Worl");
        assert_eq!(session.cursor_position, 9);

        // Test backspace at position 0 (should do nothing)
        session.cursor_position = 0;
        let original_buffer = session.input_buffer.clone();
        let _ = handle_quiz_input(&mut session, backspace_key, app_state);

        assert_eq!(session.input_buffer, original_buffer);
        assert_eq!(session.cursor_position, 0);
    }

    #[test]
    fn test_ctrl_enter_inserts_newline() {
        use tokio::sync::mpsc;

        let (tx, _rx) = mpsc::channel(32);
        let mut session = QuizSession {
            flashcards: vec![Flashcard {
                question: "Test?".to_string(),
                answer: "Answer".to_string(),
                user_answer: None,
                ai_feedback: None,
                written_to_file: false,
                id: None,
            }],
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: "Hello".to_string(),
            cursor_position: 5,
            session_id: None,
            questions_total: 1,
            questions_answered: 0,
            ai_enabled: false,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: Some(tx),
            ai_rx: None,

            input_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
        };
        let app_state = &mut AppState::Quiz;

        // Press Ctrl+Enter
        let ctrl_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL);
        let _ = handle_quiz_input(&mut session, ctrl_enter, app_state);

        // Should insert newline at cursor position
        assert_eq!(session.input_buffer, "Hello\n");
        assert_eq!(session.cursor_position, 6);
        assert!(!session.showing_answer); // Should not submit
    }

    #[test]
    fn test_ctrl_enter_in_middle_of_text() {
        use tokio::sync::mpsc;

        let (tx, _rx) = mpsc::channel(32);
        let mut session = QuizSession {
            flashcards: vec![Flashcard {
                question: "Test?".to_string(),
                answer: "Answer".to_string(),
                user_answer: None,
                ai_feedback: None,
                written_to_file: false,
                id: None,
            }],
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: "Hello world".to_string(),
            cursor_position: 5, // After "Hello"
            session_id: None,
            questions_total: 1,
            questions_answered: 0,
            ai_enabled: false,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: Some(tx),
            ai_rx: None,

            input_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
        };
        let app_state = &mut AppState::Quiz;

        // Press Ctrl+Enter
        let ctrl_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL);
        let _ = handle_quiz_input(&mut session, ctrl_enter, app_state);

        // Should insert newline in middle of text
        assert_eq!(session.input_buffer, "Hello\n world");
        assert_eq!(session.cursor_position, 6);
    }

    #[test]
    fn test_multiline_answer_submission() {
        use tokio::sync::mpsc;

        let (tx, _rx) = mpsc::channel(32);
        let mut session = QuizSession {
            flashcards: vec![Flashcard {
                question: "Test?".to_string(),
                answer: "Answer".to_string(),
                user_answer: None,
                ai_feedback: None,
                written_to_file: false,
                id: None,
            }],
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: "Line 1\nLine 2\nLine 3".to_string(),
            cursor_position: 17,
            session_id: None,
            questions_total: 1,
            questions_answered: 0,
            ai_enabled: false,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: Some(tx),
            ai_rx: None,

            input_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
        };
        let app_state = &mut AppState::Quiz;

        // Press Enter to submit
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, enter, app_state);

        // Should save multi-line answer with newlines preserved
        assert_eq!(
            session.flashcards[0].user_answer,
            Some("Line 1\nLine 2\nLine 3".to_string())
        );
        assert!(session.showing_answer); // Should show answer screen
    }

    #[test]
    fn test_cursor_position_on_question_navigation() {
        use tokio::sync::mpsc;

        let (tx, _rx) = mpsc::channel(32);
        let mut session = QuizSession {
            flashcards: vec![
                Flashcard {
                    question: "Q1?".to_string(),
                    answer: "A1".to_string(),
                    user_answer: Some("Answer1".to_string()),
                    ai_feedback: None,
                    written_to_file: false,
                    id: None,
                },
                Flashcard {
                    question: "Q2?".to_string(),
                    answer: "A2".to_string(),
                    user_answer: Some("Answer2".to_string()),
                    ai_feedback: None,
                    written_to_file: false,
                    id: None,
                },
            ],
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
            cursor_position: 0,
            session_id: None,
            questions_total: 2,
            questions_answered: 0,
            ai_enabled: false,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: Some(tx),
            ai_rx: None,

            input_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
        };
        let app_state = &mut AppState::Quiz;

        // Navigate to next question (Down arrow) - both questions are answered, so should show answer screen
        let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, down_key, app_state);

        assert_eq!(session.current_index, 1);
        assert!(session.showing_answer); // Should be in answer mode for answered question

        // Navigate back (Up arrow) - should also show answer screen
        let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, up_key, app_state);

        assert_eq!(session.current_index, 0);
        assert!(session.showing_answer); // Should be in answer mode for answered question
    }

    #[test]
    fn test_cursor_edge_cases() {
        use tokio::sync::mpsc;

        let (tx, _rx) = mpsc::channel(32);
        let mut session = QuizSession {
            flashcards: vec![Flashcard {
                question: "Test?".to_string(),
                answer: "Answer".to_string(),
                user_answer: None,
                ai_feedback: None,
                written_to_file: false,
                id: None,
            }],
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
            cursor_position: 0,
            session_id: None,
            questions_total: 1,
            questions_answered: 0,
            ai_enabled: false,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: Some(tx),
            ai_rx: None,

            input_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
        };
        let app_state = &mut AppState::Quiz;

        // Test with empty buffer: left/right arrows should do nothing
        let left_key = KeyEvent::new(KeyCode::Left, KeyModifiers::empty());
        let right_key = KeyEvent::new(KeyCode::Right, KeyModifiers::empty());

        let _ = handle_quiz_input(&mut session, left_key, app_state);
        assert_eq!(session.cursor_position, 0);

        let _ = handle_quiz_input(&mut session, right_key, app_state);
        assert_eq!(session.cursor_position, 0);

        // Add some text and test bounds
        let h_key = KeyEvent::new(KeyCode::Char('H'), KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, h_key, app_state);
        assert_eq!(session.input_buffer, "H");
        assert_eq!(session.cursor_position, 1);

        // Cursor should be constrained to valid range
        session.cursor_position = 10; // Invalid position
        let _ = handle_quiz_input(&mut session, left_key, app_state);
        assert_eq!(session.cursor_position, 1); // Should be at valid max (length)

        // Test backspace on single character
        let backspace_key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, backspace_key, app_state);
        assert_eq!(session.input_buffer, "");
        assert_eq!(session.cursor_position, 0);
    }

    #[test]
    fn test_navigation_shows_answer_screen_for_answered_questions() {
        use tokio::sync::mpsc;

        let (tx, _rx) = mpsc::channel(32);
        let mut session = QuizSession {
            flashcards: vec![
                Flashcard {
                    question: "Q1?".to_string(),
                    answer: "A1".to_string(),
                    user_answer: Some("User A1".to_string()),
                    ai_feedback: Some(crate::ai::AIFeedback {
                        is_correct: true,
                        correctness_score: 1.0,
                        corrections: vec![],
                        explanation: "Correct!".to_string(),
                        suggestions: vec![],
                    }),
                    written_to_file: false,
                    id: None,
                },
                Flashcard {
                    question: "Q2?".to_string(),
                    answer: "A2".to_string(),
                    user_answer: None, // Unanswered
                    ai_feedback: None,
                    written_to_file: false,
                    id: None,
                },
                Flashcard {
                    question: "Q3?".to_string(),
                    answer: "A3".to_string(),
                    user_answer: Some("User A3".to_string()),
                    ai_feedback: Some(crate::ai::AIFeedback {
                        is_correct: false,
                        correctness_score: 0.5,
                        corrections: vec!["Correction".to_string()],
                        explanation: "Partial".to_string(),
                        suggestions: vec!["Suggestion".to_string()],
                    }),
                    written_to_file: false,
                    id: None,
                },
            ],
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: true, // Start on answer screen of Q1
            input_buffer: String::new(),
            cursor_position: 0,
            session_id: None,
            questions_total: 3,
            questions_answered: 2,
            ai_enabled: true,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: Some(tx),
            ai_rx: None,

            input_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
        };
        let app_state = &mut AppState::Quiz;

        // Navigate to Q2 (unanswered) - should switch to input mode and restore empty buffer
        let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, down_key, app_state);

        assert_eq!(session.current_index, 1);
        assert!(!session.showing_answer); // Should be in input mode for unanswered question
        assert_eq!(session.input_buffer, ""); // Should be empty for unanswered question
        assert_eq!(session.cursor_position, 0);

        // Navigate to Q3 (answered) - should switch to answer mode
        let _ = handle_quiz_input(&mut session, down_key, app_state);

        assert_eq!(session.current_index, 2);
        assert!(session.showing_answer); // Should be in answer mode for answered question
                                         // input_buffer should not be restored since we're in answer mode

        // Navigate back to Q2 (unanswered) - should switch to input mode
        let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        let _ = handle_quiz_input(&mut session, up_key, app_state);

        assert_eq!(session.current_index, 1);
        assert!(!session.showing_answer); // Should be in input mode for unanswered question
        assert_eq!(session.input_buffer, ""); // Should be empty

        // Navigate back to Q1 (answered) - should switch to answer mode
        let _ = handle_quiz_input(&mut session, up_key, app_state);

        assert_eq!(session.current_index, 0);
        assert!(session.showing_answer); // Should be in answer mode for answered question
    }
}
