use crate::ai::AIFeedback;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

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
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Menu,
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
