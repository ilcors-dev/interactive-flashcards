use openrouter_api::{
    models::provider_preferences::ProviderPreferences,
    models::provider_preferences::ProviderSort,
    types::chat::{ChatCompletionRequest, Message},
};
use serde::Serialize;

pub const DEFAULT_MODEL: &str = "openai/gpt-oss-120b";
pub const DEFAULT_TEMPERATURE: f32 = 0.3;
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

#[derive(Debug)]
pub struct OpenRouterClient {
    client: openrouter_api::OpenRouterClient<openrouter_api::Ready>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelConfig {
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

impl OpenRouterClient {
    pub fn new() -> Result<Self, String> {
        let client = openrouter_api::OpenRouterClient::quick()
            .map_err(|e| format!("Failed to create OpenRouter client: {}", e))?;

        Ok(Self { client })
    }

    pub async fn evaluate_answer(
        &self,
        question: &str,
        correct_answer: &str,
        user_answer: &str,
        config: Option<&ModelConfig>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let prompt = format!(
            r#"Evaluate this answer and respond ONLY with valid JSON.

Question: {}
Correct Answer: {}
User's Answer: {}

IMPORTANT:

- Respond ONLY with this exact JSON structure (no markdown, no extra text):
{{
    "is_correct": boolean,
    "correctness_score": float between 0.0 and 1.0,
    "corrections": ["correction1", "correction2"],
    "explanation": "detailed explanation. must contain also deep dives on the topic regardless of correctness",
    "suggestions": ["suggestion1", "suggestion2"]
}}
- Do not account for minor typos in the user's answer when determining correctness.
"#,
            question, correct_answer, user_answer
        );

        let model = config
            .map(|c| c.model.clone())
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());

        let messages = vec![
            Message::text(
                "system",
                "You are an educational assistant evaluating quiz answers. Be concise and helpful.",
            ),
            Message::text("user", &prompt),
        ];

        let provider = ProviderPreferences::new().with_sort(ProviderSort::Throughput);

        let request = ChatCompletionRequest {
            model,
            messages,
            provider: Some(provider),
            stream: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            models: None,
            transforms: None,
            route: None,
            user: None,
            max_tokens: config.and_then(|c| c.max_tokens),
            temperature: config.and_then(|c| c.temperature),
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            repetition_penalty: None,
            min_p: None,
            top_a: None,
            seed: None,
            stop: None,
            logit_bias: None,
            logprobs: None,
            top_logprobs: None,
            prediction: None,
            parallel_tool_calls: None,
            verbosity: None,
        };

        let response = self
            .client
            .chat()?
            .chat_completion(request)
            .await
            .map_err(|e| format!("OpenRouter API error: {}", e))?;

        if let Some(choice) = response.choices.first() {
            match &choice.message.content {
                openrouter_api::MessageContent::Text(text) => Ok(text.clone()),
                openrouter_api::MessageContent::Parts(parts) => {
                    let text_parts: Vec<String> = parts
                        .iter()
                        .filter_map(|p| {
                            if let openrouter_api::ContentPart::Text(tc) = p {
                                Some(tc.text.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    Ok(text_parts.join("\n"))
                }
            }
        } else {
            Err("No response choices received".into())
        }
    }
}
