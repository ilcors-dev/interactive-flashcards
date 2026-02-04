use crate::ai::{evaluate_answer, OpenRouterClient};
use crate::logger;
use crate::models::{AiRequest, AiResponse};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::{timeout, Duration};

const CHAT_TIMEOUT_SECS: u64 = 30;

pub fn spawn_ai_worker(
    ai_tx: Sender<AiResponse>,
    mut ai_rx: Receiver<AiRequest>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        logger::log("AI worker started (async)");
        while let Some(request) = ai_rx.recv().await {
            match request {
                AiRequest::Evaluate {
                    flashcard_index,
                    question,
                    correct_answer,
                    user_answer,
                } => {
                    logger::log(&format!(
                        "Worker received request for flashcard {}",
                        flashcard_index
                    ));

                    let client = match OpenRouterClient::new() {
                        Ok(client) => client,
                        Err(e) => {
                            let _ = ai_tx
                                .send(AiResponse::Error {
                                    flashcard_index,
                                    error: format!("Failed to create AI client: {}", e),
                                })
                                .await;
                            continue;
                        }
                    };

                    // Add network timeout handling
                    let evaluation_future =
                        evaluate_answer(&client, &question, &correct_answer, &user_answer);

                    match timeout(Duration::from_secs(30), evaluation_future).await {
                        Ok(Ok(eval_result)) => {
                            logger::log("Worker sending evaluation success");
                            let _ = ai_tx
                                .send(AiResponse::Evaluation {
                                    flashcard_index,
                                    result: eval_result,
                                })
                                .await;
                        }
                        Ok(Err(e)) => {
                            logger::log(&format!("Worker evaluation error: {}", e));
                            let full_error = format!("AI evaluation failed: {}", e);
                            let _ = ai_tx
                                .send(AiResponse::Error {
                                    flashcard_index,
                                    error: full_error,
                                })
                                .await;
                        }
                        Err(_) => {
                            logger::log("Worker timeout error");
                            let timeout_error =
                                "AI evaluation timed out after 30 seconds - press Ctrl+E to retry"
                                    .to_string();
                            let _ = ai_tx
                                .send(AiResponse::Error {
                                    flashcard_index,
                                    error: timeout_error,
                                })
                                .await;
                        }
                    }
                }
                AiRequest::EvaluateSession {
                    session_id,
                    deck_name,
                    flashcards,
                } => {
                    logger::log(&format!(
                        "Worker received session assessment request for session {}",
                        session_id
                    ));

                    let client = match OpenRouterClient::new() {
                        Ok(client) => client,
                        Err(e) => {
                            let _ = ai_tx
                                .send(AiResponse::SessionAssessment {
                                    session_id,
                                    result: Err(format!("Failed to create AI client: {}", e)),
                                })
                                .await;
                            continue;
                        }
                    };

                    let evaluation_future = client.evaluate_session(&deck_name, &flashcards, None);

                    match timeout(Duration::from_secs(60), evaluation_future).await {
                        Ok(Ok(eval_result)) => {
                            logger::log("Worker sending session assessment success");
                            let assessment = crate::ai::parse_session_assessment(&eval_result);
                            let _ = ai_tx
                                .send(AiResponse::SessionAssessment {
                                    session_id,
                                    result: assessment,
                                })
                                .await;
                        }
                        Ok(Err(e)) => {
                            logger::log(&format!("Worker session assessment error: {}", e));
                            let full_error = format!("Session assessment failed: {}", e);
                            let _ = ai_tx
                                .send(AiResponse::SessionAssessment {
                                    session_id,
                                    result: Err(full_error),
                                })
                                .await;
                        }
                        Err(_) => {
                            logger::log("Worker session assessment timeout error");
                            let timeout_error =
                                "Session assessment timed out after 60 seconds".to_string();
                            let _ = ai_tx
                                .send(AiResponse::SessionAssessment {
                                    session_id,
                                    result: Err(timeout_error),
                                })
                                .await;
                        }
                    }
                }
                AiRequest::Chat {
                    flashcard_id,
                    session_id: _,
                    question,
                    correct_answer,
                    user_answer,
                    initial_feedback,
                    conversation_history,
                    user_message,
                } => {
                    logger::log(&format!(
                        "Worker received chat request for flashcard {}",
                        flashcard_id
                    ));

                    let client = match OpenRouterClient::new() {
                        Ok(client) => client,
                        Err(e) => {
                            let _ = ai_tx
                                .send(AiResponse::ChatReply {
                                    flashcard_id,
                                    message: None,
                                    error: Some(format!("Failed to create AI client: {}", e)),
                                })
                                .await;
                            continue;
                        }
                    };

                    let chat_future = client.chat(
                        &question,
                        &correct_answer,
                        &user_answer,
                        &initial_feedback,
                        &conversation_history,
                        &user_message,
                    );

                    match timeout(Duration::from_secs(CHAT_TIMEOUT_SECS), chat_future).await {
                        Ok(Ok(reply)) => {
                            logger::log("Worker sending chat reply success");
                            let _ = ai_tx
                                .send(AiResponse::ChatReply {
                                    flashcard_id,
                                    message: Some(reply),
                                    error: None,
                                })
                                .await;
                        }
                        Ok(Err(e)) => {
                            logger::log(&format!("Worker chat error: {}", e));
                            let _ = ai_tx
                                .send(AiResponse::ChatReply {
                                    flashcard_id,
                                    message: None,
                                    error: Some(format!("Chat failed: {}", e)),
                                })
                                .await;
                        }
                        Err(_) => {
                            logger::log("Worker chat timeout");
                            let _ = ai_tx
                                .send(AiResponse::ChatReply {
                                    flashcard_id,
                                    message: None,
                                    error: Some(
                                        "Chat response timed out after 30 seconds".to_string(),
                                    ),
                                })
                                .await;
                        }
                    }
                }
            }
        }
        logger::log("AI worker exiting (channel closed)");
    })
}
