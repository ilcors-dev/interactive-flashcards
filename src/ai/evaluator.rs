use crate::ai::client::OpenRouterClient;
use serde::{Deserialize, Serialize};

fn clean_json_response(response: &str) -> String {
    let mut cleaned = response.trim().to_string();

    if cleaned.starts_with("```") {
        let lines: Vec<&str> = cleaned.lines().collect();
        if lines.len() > 2 {
            cleaned = lines[1..lines.len() - 1].join("\n");
        }
    }

    if let Some(start) = cleaned.find('{')
        && let Some(end) = cleaned.rfind('}') {
            cleaned = cleaned[start..=end].to_string();
        }

    cleaned.trim().to_string()
}

/// AI feedback for flashcard answers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIFeedback {
    pub is_correct: bool,
    pub correctness_score: f32,
    pub corrections: Vec<String>,
    pub explanation: String,
    pub suggestions: Vec<String>,
}

/// Complete AI evaluation result with raw response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIEvaluationResult {
    pub feedback: AIFeedback,
    pub raw_response: String,
}

/// Evaluate user's answer against correct answer using AI
pub async fn evaluate_answer(
    client: &OpenRouterClient,
    question: &str,
    correct_answer: &str,
    user_answer: &str,
) -> Result<AIEvaluationResult, Box<dyn std::error::Error>> {
    crate::logger::log("Starting AI evaluation");
    let json_response = client
        .evaluate_answer(question, correct_answer, user_answer, None)
        .await?;

    crate::logger::log(&format!("Raw AI response: {}", json_response));
    let cleaned = clean_json_response(&json_response);

    crate::logger::log(&format!("Cleaned AI response: {}", cleaned));

    let feedback: AIFeedback = serde_json::from_str(&cleaned).map_err(|e| {
        format!(
            "Failed to parse AI response as JSON: {}\nRaw: {}\nCleaned: {}",
            e, json_response, cleaned
        )
    })?;

    if feedback.correctness_score < 0.0 || feedback.correctness_score > 1.0 {
        return Err(format!(
            "Invalid correctness score: {}. Raw: {}",
            feedback.correctness_score, json_response
        )
        .into());
    }

    Ok(AIEvaluationResult {
        feedback,
        raw_response: json_response,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_json_response_simple() {
        let json = r#"{"is_correct":true}"#;
        let cleaned = clean_json_response(json);
        assert_eq!(cleaned, r#"{"is_correct":true}"#);
    }

    #[test]
    fn test_clean_json_response_markdown() {
        let json = r#"```json
{"is_correct": true, "correctness_score": 0.9}
```"#;
        let cleaned = clean_json_response(json);
        assert_eq!(cleaned, r#"{"is_correct": true, "correctness_score": 0.9}"#);
    }

    #[test]
    fn test_clean_json_response_with_text() {
        let json = r#"Here's your response: {"is_correct": true, "score": 0.9} thanks"#;
        let cleaned = clean_json_response(json);
        assert_eq!(cleaned, r#"{"is_correct": true, "score": 0.9}"#);
    }

    #[test]
    fn test_parse_valid_feedback() {
        let json = r#"{
            "is_correct": false,
            "correctness_score": 0.75,
            "corrections": ["Missed concept X"],
            "explanation": "Here's why...",
            "suggestions": ["Try this approach"]
        }"#;

        let feedback: AIFeedback = serde_json::from_str(json).unwrap();
        assert_eq!(feedback.is_correct, false);
        assert_eq!(feedback.correctness_score, 0.75);
        assert_eq!(feedback.corrections, vec!["Missed concept X".to_string()]);
    }

    #[test]
    fn test_parse_invalid_score() {
        let json = r#"{
            "is_correct": true,
            "correctness_score": 1.5
        }"#;

        let result: Result<AIFeedback, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
