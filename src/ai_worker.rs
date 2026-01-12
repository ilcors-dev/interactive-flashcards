use crate::ai::{evaluate_answer, OpenRouterClient};
use crate::logger;
use crate::models::{AiRequest, AiResponse};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::{timeout, Duration};

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
            }
        }
        logger::log("AI worker exiting (channel closed)");
    })
}
