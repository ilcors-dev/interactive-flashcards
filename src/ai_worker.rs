use crate::ai::{evaluate_answer, OpenRouterClient};
use crate::logger;
use crate::models::{AiRequest, AiResponse};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

pub fn spawn_ai_worker(
    ai_tx: Sender<AiResponse>,
    ai_rx: Receiver<AiRequest>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("interactive-flashcards::ai_worker".to_string())
        .spawn(move || loop {
            match ai_rx.recv() {
                Ok(AiRequest::Evaluate {
                    flashcard_index,
                    question,
                    correct_answer,
                    user_answer,
                }) => {
                    logger::log(&format!(
                        "Worker received request for flashcard {}",
                        flashcard_index
                    ));
                    let client = match OpenRouterClient::new() {
                        Ok(client) => client,
                        Err(e) => {
                            let _ = ai_tx.send(AiResponse::Error {
                                flashcard_index,
                                error: format!("Failed to create AI client: {}", e),
                            });
                            continue;
                        }
                    };

                    let rt = tokio::runtime::Runtime::new().unwrap();

                    let result = rt.block_on(async {
                        evaluate_answer(&client, &question, &correct_answer, &user_answer).await
                    });

                    match result {
                        Ok(eval_result) => {
                            logger::log("Worker sending evaluation success");
                            let _ = ai_tx.send(AiResponse::Evaluation {
                                flashcard_index,
                                result: eval_result,
                            });
                        }
                        Err(e) => {
                            logger::log(&format!("Worker error: {}", e));
                            let full_error = format!("AI evaluation failed: {}", e);
                            let _ = ai_tx.send(AiResponse::Error {
                                flashcard_index,
                                error: full_error,
                            });
                        }
                    }
                }
                Err(_) => {
                    // Channel disconnected, exit worker
                    logger::log("Worker channel disconnected, exiting");
                    break;
                }
            }
        })
        .expect("Failed to spawn AI worker thread")
}
