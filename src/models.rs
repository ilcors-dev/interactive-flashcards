#[derive(Debug, Clone)]
pub struct Flashcard {
    pub question: String,
    pub answer: String,
    pub user_answer: Option<String>,
}

#[derive(Debug)]
pub struct QuizSession {
    pub flashcards: Vec<Flashcard>,
    pub current_index: usize,
    pub deck_name: String,
    pub showing_answer: bool,
    pub input_buffer: String,
    pub output_file: Option<std::fs::File>,
    pub questions_total: usize,
    pub questions_answered: usize,
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
            },
            Flashcard {
                question: "Q2".to_string(),
                answer: "A2".to_string(),
                user_answer: None,
            },
        ];
        let session = QuizSession {
            flashcards: cards.clone(),
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
            output_file: None,
            questions_total: cards.len(),
            questions_answered: 0,
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
        }];
        let mut session = QuizSession {
            flashcards: cards.clone(),
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
            output_file: None,
            questions_total: cards.len(),
            questions_answered: 0,
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
            },
            Flashcard {
                question: "Q2".to_string(),
                answer: "A2".to_string(),
                user_answer: None,
            },
            Flashcard {
                question: "Q3".to_string(),
                answer: "A3".to_string(),
                user_answer: None,
            },
        ];
        let mut session = QuizSession {
            flashcards: cards.clone(),
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
            output_file: None,
            questions_total: cards.len(),
            questions_answered: 0,
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
                user_answer: None,
            },
            Flashcard {
                question: "Q2".to_string(),
                answer: "A2".to_string(),
                user_answer: None,
            },
        ];

        cards[0].user_answer = Some("Answer 1".to_string());
        cards[1].user_answer = Some("Answer 2".to_string());

        let session = QuizSession {
            flashcards: cards.clone(),
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
            output_file: None,
            questions_total: cards.len(),
            questions_answered: 0,
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
