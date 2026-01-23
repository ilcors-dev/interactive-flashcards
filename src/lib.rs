pub mod ai;
pub mod ai_worker;
pub mod csv;
pub mod db;
pub mod file_io;
pub mod logger;
pub mod models;
pub mod session;
pub mod ui;
pub mod ui_tests;
pub mod utils;

// Re-exports for convenience
pub use ai::{
    evaluate_answer, AIEvaluationResult, AIFeedback, ModelConfig, OpenRouterClient, DEFAULT_MODEL,
};
pub use csv::{get_csv_files, load_csv};
pub use db::flashcard;
pub use models::{AppState, Flashcard, QuizSession};
pub use session::handle_quiz_input;
pub use ui::{draw_menu, draw_quit_confirmation, draw_quiz, draw_summary};
pub use utils::{calculate_wrapped_cursor_position, render_markdown, render_markdown_truncated};
