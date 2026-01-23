use crate::ai::client::OpenRouterClient;
use crate::models::SessionAssessment;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionAssessmentRaw {
    grade_percentage: f32,
    mastery_level: String,
    overall_feedback: String,
    suggestions: Vec<String>,
    strengths: Vec<String>,
    weaknesses: Vec<String>,
}

pub fn parse_session_assessment(response: &str) -> Result<SessionAssessment, String> {
    let cleaned = clean_json_response(response);
    let raw: SessionAssessmentRaw = serde_json::from_str(&cleaned).map_err(|e| {
        format!(
            "Failed to parse session assessment: {}\nRaw: {}\nCleaned: {}",
            e, response, cleaned
        )
    })?;

    Ok(SessionAssessment {
        grade_percentage: raw.grade_percentage,
        mastery_level: raw.mastery_level,
        overall_feedback: raw.overall_feedback,
        suggestions: raw.suggestions,
        strengths: raw.strengths,
        weaknesses: raw.weaknesses,
    })
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
) -> Result<AIEvaluationResult, Box<dyn std::error::Error + Send + Sync>> {
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
use std::time::Duration;
#[cfg(test)]
use tokio::time::sleep;

/// Mock AI client for testing - simulates AI responses with configurable delays
#[cfg(test)]
pub struct MockAiClient {
    responses: Vec<AIEvaluationResult>,
    delays: Vec<Duration>,
    current_index: usize,
}

#[cfg(test)]
impl MockAiClient {
    /// Create a new mock client with default successful responses
    pub fn new() -> Self {
        Self {
            responses: vec![
                AIEvaluationResult {
                    feedback: AIFeedback {
                        is_correct: true,
                        correctness_score: 1.0,
                        corrections: vec![],
                        explanation: "Perfect answer! Well done.".to_string(),
                        suggestions: vec![],
                    },
                    raw_response: r#"{"is_correct": true, "correctness_score": 1.0, "corrections": [], "explanation": "Perfect answer! Well done.", "suggestions": []}"#.to_string(),
                },
                AIEvaluationResult {
                    feedback: AIFeedback {
                        is_correct: false,
                        correctness_score: 0.6,
                        corrections: vec!["Incorrect terminology".to_string()],
                        explanation: "Good attempt, but there's an error in the terminology.".to_string(),
                        suggestions: vec!["Review the key terms".to_string()],
                    },
                    raw_response: r#"{"is_correct": false, "correctness_score": 0.6, "corrections": ["Incorrect terminology"], "explanation": "Good attempt, but there's an error in the terminology.", "suggestions": ["Review the key terms"]}"#.to_string(),
                },
            ],
            delays: vec![Duration::from_millis(50), Duration::from_millis(75)],
            current_index: 0,
        }
    }

    /// Create mock client with custom responses and delays
    pub fn with_responses(responses: Vec<AIEvaluationResult>, delays: Vec<Duration>) -> Self {
        Self {
            responses,
            delays,
            current_index: 0,
        }
    }

    /// Simulate AI evaluation with delay
    pub async fn evaluate_answer(
        &mut self,
        _question: &str,
        _correct_answer: &str,
        _user_answer: &str,
    ) -> Result<AIEvaluationResult, Box<dyn std::error::Error + Send + Sync>> {
        // Simulate network delay
        if self.current_index < self.delays.len() {
            sleep(self.delays[self.current_index]).await;
        }

        // Return configured response
        if self.current_index < self.responses.len() {
            let response = self.responses[self.current_index].clone();
            self.current_index += 1;
            Ok(response)
        } else {
            // Cycle back to first response if we run out
            self.current_index = 0;
            Ok(self.responses[0].clone())
        }
    }

    /// Reset the mock client state
    pub fn reset(&mut self) {
        self.current_index = 0;
    }
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

    #[test]
    fn test_parse_session_assessment() {
        let json = r#"{
            "grade_percentage": 85.0,
            "mastery_level": "Intermediate",
            "overall_feedback": "Great progress on the fundamentals.",
            "suggestions": ["Review chapter 3", "Practice more examples"],
            "strengths": ["Core concepts", "Terminology"],
            "weaknesses": ["Application questions"]
        }"#;

        let result = parse_session_assessment(json);
        assert!(result.is_ok());
        let assessment = result.unwrap();
        assert_eq!(assessment.grade_percentage, 85.0);
        assert_eq!(assessment.mastery_level, "Intermediate");
        assert!(assessment.suggestions.len() == 2);
        assert!(assessment.strengths.len() == 2);
        assert!(assessment.weaknesses.len() == 1);
    }

    #[test]
    fn test_parse_session_assessment_with_markdown() {
        let json = r#"```json
{
    "grade_percentage": 70.5,
    "mastery_level": "Intermediate",
    "overall_feedback": "Good effort.",
    "suggestions": ["Keep practicing"],
    "strengths": ["Good start"],
    "weaknesses": ["Need more review"]
}
```"#;

        let result = parse_session_assessment(json);
        assert!(result.is_ok());
        let assessment = result.unwrap();
        assert_eq!(assessment.grade_percentage, 70.5);
    }
}
