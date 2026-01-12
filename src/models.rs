use crate::ai::AIFeedback;

#[derive(Debug, Clone)]
pub struct Flashcard {
    pub question: String,
    pub answer: String,
    pub user_answer: Option<String>,
    pub ai_feedback: Option<AIFeedback>,
    pub written_to_file: bool,
}

#[derive(Debug)]
pub struct QuizSession {
    pub flashcards: Vec<Flashcard>,
    pub current_index: usize,
    pub deck_name: String,
    pub showing_answer: bool,
    pub input_buffer: String,
    pub cursor_position: usize,
    pub output_file: Option<std::fs::File>,
    pub questions_total: usize,
    pub questions_answered: usize,
    pub ai_enabled: bool,
    pub ai_evaluation_in_progress: bool,
    pub ai_last_evaluated_index: Option<usize>,
    pub ai_evaluation_start_time: Option<std::time::Instant>,
    pub last_ai_error: Option<String>,
    pub ai_tx: Option<std::sync::mpsc::Sender<AiRequest>>,
    pub ai_rx: Option<std::sync::mpsc::Receiver<AiResponse>>,
    pub progress_header_position: u64,
    pub input_scroll_y: u16,
}

#[derive(Debug)]
pub enum AiRequest {
    Evaluate {
        flashcard_index: usize,
        question: String,
        correct_answer: String,
        user_answer: String,
    },
}

#[derive(Debug)]
pub enum AiResponse {
    Evaluation {
        flashcard_index: usize,
        result: crate::ai::AIEvaluationResult,
    },
    Error {
        flashcard_index: usize,
        error: String,
    },
}

#[derive(Debug, PartialEq)]
pub enum AppState {
    Menu,
    Quiz,
    QuizQuitConfirm,
    Summary,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flashcard_creation() {
        let card = Flashcard {
            question: "Question?".to_string(),
            answer: "Answer".to_string(),
            user_answer: None,
            ai_feedback: None,
            written_to_file: false,
        };

        assert_eq!(card.question, "Question?");
        assert_eq!(card.answer, "Answer");
        assert!(card.user_answer.is_none());
    }

    #[test]
    fn test_flashcard_with_user_answer() {
        let card = Flashcard {
            question: "Question?".to_string(),
            answer: "Answer".to_string(),
            user_answer: Some("My Answer".to_string()),
            ai_feedback: None,
            written_to_file: false,
        };
        assert_eq!(card.question, "Question?");
        assert_eq!(card.answer, "Answer");
        assert!(card.user_answer.is_some());
        assert_eq!(card.user_answer.unwrap(), "My Answer");
    }

    #[test]
    fn test_flashcard_clone() {
        let card = Flashcard {
            question: "Q".to_string(),
            answer: "A".to_string(),
            user_answer: Some("UA".to_string()),
            ai_feedback: None,
            written_to_file: false,
        };
        let cloned = card.clone();
        assert_eq!(card.question, cloned.question);
        assert_eq!(card.answer, cloned.answer);
        assert_eq!(card.user_answer, cloned.user_answer);
    }

    #[test]
    fn test_quiz_session_creation() {
        let cards = vec![
            Flashcard {
                question: "Q1".to_string(),
                answer: "A1".to_string(),
                user_answer: None,
                ai_feedback: None,
                written_to_file: false,
            },
            Flashcard {
                question: "Q2".to_string(),
                answer: "A2".to_string(),
                user_answer: None,
                ai_feedback: None,
                written_to_file: false,
            },
        ];
        let mut session = QuizSession {
            flashcards: cards.clone(),
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
            cursor_position: 0,
            output_file: None,
            questions_total: cards.len(),
            questions_answered: 0,
            ai_enabled: false,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: None,
            ai_rx: None,
            progress_header_position: 0,
            input_scroll_y: 0,
        };
        assert_eq!(session.flashcards.len(), 2);
        assert_eq!(session.current_index, 0);
        assert_eq!(session.deck_name, "Test");
        assert!(!session.showing_answer);
        assert!(session.input_buffer.is_empty());
    }

    #[test]
    fn test_quiz_session_state_transitions() {
        let cards = vec![Flashcard {
            question: "Q1".to_string(),
            answer: "A1".to_string(),
            user_answer: None,
            ai_feedback: None,
            written_to_file: false,
        }];
        let mut session = QuizSession {
            flashcards: cards.clone(),
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
            cursor_position: 0,
            output_file: None,
            questions_total: cards.len(),
            questions_answered: 0,
            ai_enabled: false,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: None,
            ai_rx: None,
            progress_header_position: 0,
            input_scroll_y: 0,
        };

        session.showing_answer = true;
        assert!(session.showing_answer);

        session.flashcards[0].user_answer = Some("My Answer".to_string());
        assert!(session.flashcards[0].user_answer.is_some());
    }

    #[test]
    fn test_multiple_flashcards_navigation() {
        let cards = vec![
            Flashcard {
                question: "Q1".to_string(),
                answer: "A1".to_string(),
                user_answer: None,
                ai_feedback: None,
                written_to_file: false,
            },
            Flashcard {
                question: "Q2".to_string(),
                answer: "A2".to_string(),
                user_answer: None,
                ai_feedback: None,
                written_to_file: false,
            },
            Flashcard {
                question: "Q3".to_string(),
                answer: "A3".to_string(),
                user_answer: None,
                ai_feedback: None,
                written_to_file: false,
            },
        ];
        let mut session = QuizSession {
            flashcards: cards.clone(),
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
            cursor_position: 0,
            output_file: None,
            questions_total: cards.len(),
            questions_answered: 0,
            ai_enabled: false,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: None,
            ai_rx: None,
            progress_header_position: 0,
            input_scroll_y: 0,
        };

        assert_eq!(session.current_index, 0);
        session.current_index += 1;
        assert_eq!(session.current_index, 1);
        session.current_index += 1;
        assert_eq!(session.current_index, 2);
        session.current_index -= 1;
        assert_eq!(session.current_index, 1);
    }

    #[test]
    fn test_state_preservation_on_navigation() {
        let mut cards = vec![
            Flashcard {
                question: "Q1".to_string(),
                answer: "A1".to_string(),
                user_answer: Some("Answer 1".to_string()),
                ai_feedback: None,
                written_to_file: false,
            },
            Flashcard {
                question: "Q2".to_string(),
                answer: "A2".to_string(),
                user_answer: Some("Answer 2".to_string()),
                ai_feedback: None,
                written_to_file: false,
            },
            Flashcard {
                question: "Q3".to_string(),
                answer: "A3".to_string(),
                user_answer: None,
                ai_feedback: None,
                written_to_file: false,
            },
        ];
        let mut session = QuizSession {
            flashcards: cards.clone(),
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
            cursor_position: 0,
            output_file: None,
            questions_total: cards.len(),
            questions_answered: 0,
            ai_enabled: false,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: None,
            ai_rx: None,
            progress_header_position: 0,
            input_scroll_y: 0,
        };

        assert_eq!(
            session.flashcards[0].user_answer,
            Some("Answer 1".to_string())
        );
        assert_eq!(
            session.flashcards[1].user_answer,
            Some("Answer 2".to_string())
        );
    }

    #[test]
    fn test_app_state_transitions() {
        let mut state = AppState::Menu;
        assert_eq!(state, AppState::Menu);

        state = AppState::Quiz;
        assert_eq!(state, AppState::Quiz);

        state = AppState::Summary;
        assert_eq!(state, AppState::Summary);

        state = AppState::Menu;
        assert_eq!(state, AppState::Menu);
    }
}
