use crate::ai::AIFeedback;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone, PartialEq)]
pub enum ChatRole {
    User,
    Assistant,
    System,
}

impl ChatRole {
    pub fn as_str(&self) -> &str {
        match self {
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
            ChatRole::System => "system",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "user" => ChatRole::User,
            "assistant" => ChatRole::Assistant,
            "system" => ChatRole::System,
            _ => ChatRole::User,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: Option<u64>,
    pub role: ChatRole,
    pub content: String,
    pub message_order: u32,
}

#[derive(Debug)]
pub struct ChatState {
    pub flashcard_id: u64,
    pub session_id: u64,
    pub messages: Vec<ChatMessage>,
    pub input_buffer: String,
    pub cursor_position: usize,
    pub scroll_y: u16,
    pub is_loading: bool,
    pub error: Option<String>,
    pub read_only: bool,
    /// Cached rendered lines for display - rebuilt only when messages change
    pub rendered_lines_cache: Vec<ratatui::text::Line<'static>>,
    /// Track message count to know when to invalidate cache
    pub cached_message_count: usize,
    /// Cached max scroll value from last render - used for bounds checking in event handlers
    pub max_scroll: u16,
}

#[derive(Debug, Clone)]
pub struct Flashcard {
    pub question: String,
    pub answer: String,
    pub user_answer: Option<String>,
    pub ai_feedback: Option<AIFeedback>,
    pub written_to_file: bool,
    pub id: Option<u64>,
}

#[derive(Debug)]
pub struct QuizSession {
    pub flashcards: Vec<Flashcard>,
    pub current_index: usize,
    pub deck_name: String,
    pub showing_answer: bool,
    pub input_buffer: String,
    pub cursor_position: usize,
    pub session_id: Option<u64>,
    pub questions_total: usize,
    pub questions_answered: usize,
    pub ai_enabled: bool,
    pub ai_evaluation_in_progress: bool,
    pub ai_last_evaluated_index: Option<usize>,
    pub ai_evaluation_start_time: Option<std::time::Instant>,
    pub last_ai_error: Option<String>,
    pub ai_tx: Option<mpsc::Sender<AiRequest>>,
    pub ai_rx: Option<mpsc::Receiver<AiResponse>>,
    pub input_scroll_y: u16,
    pub feedback_scroll_y: u16,
    pub session_assessment: Option<SessionAssessment>,
    pub assessment_loading: bool,
    pub assessment_error: Option<String>,
    pub assessment_scroll_y: u16,
    pub chat_state: Option<ChatState>,
}

impl QuizSession {
    /// Calculate the session statistics.
    /// Returns (answered_count, average_score_percentage).
    /// Average score treats unanswered questions as 0%.
    pub fn calculate_stats(&self) -> (usize, f32) {
        if self.questions_total == 0 {
            return (0, 0.0);
        }

        let answered_count = self
            .flashcards
            .iter()
            .filter(|c| c.user_answer.is_some())
            .count();

        // Calculate sum of scores (unanswered or error = 0.0)
        let total_score: f32 = self
            .flashcards
            .iter()
            .map(|c| {
                c.ai_feedback
                    .as_ref()
                    .map(|f| f.correctness_score)
                    .unwrap_or(0.0)
            })
            .sum();

        // Average over TOTAL questions (not just answered ones)
        let average_score = total_score / self.questions_total as f32;

        (answered_count, average_score * 100.0)
    }
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

/// Async-safe wrapper for QuizSession using RwLock for concurrent access
#[derive(Debug, Clone)]
pub struct AsyncQuizSession {
    inner: Arc<RwLock<QuizSession>>,
}

impl AsyncQuizSession {
    /// Create a new async quiz session
    pub fn new(session: QuizSession) -> Self {
        Self {
            inner: Arc::new(RwLock::new(session)),
        }
    }

    /// Get a clone of the current session state (read-only)
    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, QuizSession> {
        self.inner.read().await
    }

    /// Get mutable access to the session state
    pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, QuizSession> {
        self.inner.write().await
    }

    /// Process an AI response (async-safe)
    pub async fn process_ai_response(
        &self,
        response: AiResponse,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut session = self.inner.write().await;
        session.process_ai_responses(response);
        Ok(())
    }

    /// Check if there are pending AI responses
    pub async fn has_pending_ai_responses(&self) -> bool {
        let session = self.inner.read().await;
        // For now, just check if AI evaluation is in progress
        // In a real implementation, we'd check the channel length
        session.ai_evaluation_in_progress
    }

    /// Get the current display state for UI rendering
    pub async fn get_display_state(&self) -> (AppState, Option<QuizSession>) {
        // For now, return None to indicate async mode
        // This will be refined in the main loop implementation
        (AppState::Quiz, None)
    }
}

#[derive(Debug)]
pub enum AiRequest {
    Evaluate {
        flashcard_index: usize,
        question: String,
        correct_answer: String,
        user_answer: String,
    },
    EvaluateSession {
        session_id: u64,
        deck_name: String,
        flashcards: Vec<(String, String, Option<String>, Option<AIFeedback>)>,
    },
    Chat {
        flashcard_id: u64,
        session_id: u64,
        question: String,
        correct_answer: String,
        user_answer: String,
        initial_feedback: String,
        conversation_history: Vec<(String, String)>,
        user_message: String,
    },
}

#[derive(Debug)]
pub enum AiResponse {
    Evaluation {
        flashcard_index: usize,
        result: crate::ai::AIEvaluationResult,
    },
    SessionAssessment {
        session_id: u64,
        result: Result<SessionAssessment, String>,
    },
    Error {
        flashcard_index: usize,
        error: String,
    },
    ChatReply {
        flashcard_id: u64,
        message: Option<String>,
        error: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiState {
    pub app_state: AppState,
    pub current: Option<UiStateTypes>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiStateTypes {
    Menu(UiMenuState),
    Quiz(UiQuizState),
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiMenuState {
    pub selected_file_index: usize,
    pub selected_session_index: usize,
    pub focused_panel: usize, // 0 = CSV, 1 = Sessions
    pub sessions_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiQuizState {
    pub current_index: usize,
    pub showing_answer: bool,
    pub ai_evaluation_in_progress: bool,
    pub input_buffer_len: usize,
    pub cursor_position: usize,
    pub input_scroll_y: u16,
    pub feedback_scroll_y: u16,
    pub has_ai_error: bool,
    pub questions_answered: usize,
    pub ai_feedback_count: usize,
    pub chat_open: bool,
    pub chat_message_count: usize,
    pub chat_input_len: usize,
    pub chat_is_loading: bool,
    pub chat_scroll_y: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Menu,
    MenuDeleteConfirm,
    Quiz,
    QuizQuitConfirm,
    Summary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAssessment {
    pub grade_percentage: f32,
    pub mastery_level: String,
    pub overall_feedback: String,
    pub suggestions: Vec<String>,
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SessionComparison {
    pub previous_sessions: usize,
    pub improvement_from_avg: f32,
    pub trend: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_session(flashcards: Vec<Flashcard>) -> QuizSession {
        QuizSession {
            flashcards: flashcards.clone(),
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
            cursor_position: 0,
            session_id: None,
            questions_total: flashcards.len(),
            questions_answered: 0, // This is updated during quiz, but calculate_stats relies on user_answer present
            ai_enabled: true,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: None,
            ai_rx: None,
            input_scroll_y: 0,
            feedback_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
            chat_state: None,
        }
    }

    #[test]
    fn test_calculate_stats_perfect() {
        let flashcards = vec![
            Flashcard {
                question: "Q1".to_string(),
                answer: "A1".to_string(),
                user_answer: Some("A1".to_string()),
                ai_feedback: Some(AIFeedback {
                    is_correct: true,
                    correctness_score: 1.0,
                    corrections: vec![],
                    explanation: "Good".to_string(),
                    suggestions: vec![],
                }),
                written_to_file: false,
                id: None,
            },
            Flashcard {
                question: "Q2".to_string(),
                answer: "A2".to_string(),
                user_answer: Some("A2".to_string()),
                ai_feedback: Some(AIFeedback {
                    is_correct: true,
                    correctness_score: 1.0,
                    corrections: vec![],
                    explanation: "Good".to_string(),
                    suggestions: vec![],
                }),
                written_to_file: false,
                id: None,
            },
        ];
        let session = create_test_session(flashcards);
        let (answered, score) = session.calculate_stats();
        assert_eq!(answered, 2);
        assert_eq!(score, 100.0);
    }

    #[test]
    fn test_calculate_stats_partial() {
        let flashcards = vec![
            Flashcard {
                question: "Q1".to_string(),
                answer: "A1".to_string(),
                user_answer: Some("A1".to_string()),
                ai_feedback: Some(AIFeedback {
                    is_correct: true,
                    correctness_score: 1.0,
                    corrections: vec![],
                    explanation: "Good".to_string(),
                    suggestions: vec![],
                }),
                written_to_file: false,
                id: None,
            },
            Flashcard {
                question: "Q2".to_string(),
                answer: "A2".to_string(),
                user_answer: Some("A2".to_string()),
                ai_feedback: Some(AIFeedback {
                    is_correct: false,
                    correctness_score: 0.5,
                    corrections: vec![],
                    explanation: "Partial".to_string(),
                    suggestions: vec![],
                }),
                written_to_file: false,
                id: None,
            },
        ];
        let session = create_test_session(flashcards);
        let (answered, score) = session.calculate_stats();
        assert_eq!(answered, 2);
        assert_eq!(score, 75.0); // (1.0 + 0.5) / 2 = 0.75 -> 75%
    }

    #[test]
    fn test_calculate_stats_unanswered() {
        let flashcards = vec![
            Flashcard {
                question: "Q1".to_string(),
                answer: "A1".to_string(),
                user_answer: Some("A1".to_string()),
                ai_feedback: Some(AIFeedback {
                    is_correct: true,
                    correctness_score: 1.0,
                    corrections: vec![],
                    explanation: "Good".to_string(),
                    suggestions: vec![],
                }),
                written_to_file: false,
                id: None,
            },
            Flashcard {
                question: "Q2".to_string(),
                answer: "A2".to_string(),
                user_answer: None, // Unanswered
                ai_feedback: None,
                written_to_file: false,
                id: None,
            },
        ];
        let session = create_test_session(flashcards);
        let (answered, score) = session.calculate_stats();
        assert_eq!(answered, 1);
        assert_eq!(score, 50.0); // (1.0 + 0.0) / 2 = 0.50 -> 50%
    }

    #[test]
    fn test_calculate_stats_zero_questions() {
        let flashcards = vec![];
        let session = create_test_session(flashcards);
        let (answered, score) = session.calculate_stats();
        assert_eq!(answered, 0);
        assert_eq!(score, 0.0);
    }
}
