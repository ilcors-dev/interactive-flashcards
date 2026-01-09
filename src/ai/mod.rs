pub mod client;
pub mod evaluator;

// Public API exports
pub use client::{ModelConfig, OpenRouterClient, DEFAULT_MODEL};
pub use evaluator::{evaluate_answer, AIEvaluationResult, AIFeedback};
