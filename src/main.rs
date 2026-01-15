use crossterm::{
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture, Event, EventStream, KeyCode, KeyModifiers,
        MouseEventKind,
    },
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::seq::SliceRandom;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::fs;
use std::io;

use futures::StreamExt;
use std::time::UNIX_EPOCH;
use tokio::sync::mpsc;
use tokio::time::{self, Duration};

use interactive_flashcards::{
    ai_worker, draw_menu, draw_quit_confirmation, draw_quiz, draw_summary, get_csv_files,
    handle_quiz_input, load_csv, logger,
    models::{
        AiRequest, AiResponse, AppState, QuizSession, UiMenuState, UiQuizState, UiState,
        UiStateTypes,
    },
    write_session_header,
};

fn get_quiz_filename(deck_name: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("quiz_{}_{}.txt", deck_name, timestamp)
}

#[tokio::main]
async fn main() -> io::Result<()> {
    logger::init();
    logger::log("Application started");

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableBracketedPaste,
        EnableMouseCapture,
        EnableFocusChange
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app_state = AppState::Menu;
    let csv_files = get_csv_files();
    let mut selected_file_index: usize = 0;
    let mut quiz_session: Option<QuizSession> = None;
    let ai_enabled = std::env::var("OPENROUTER_API_KEY").is_ok();

    // Create async event stream and timeout timer for event-driven architecture
    let mut event_stream = EventStream::new();
    let mut ai_timeout_interval = time::interval(Duration::from_secs(30));

    // Track UI state to avoid unnecessary redraws
    let mut last_ui_state = UiState {
        app_state: AppState::Menu,
        current: None,
    };
    let mut is_first_draw = true; // Ensure UI draws on application startup

    loop {
        // Check if UI needs updating based on state changes
        let current_ui_state = match app_state {
            AppState::Menu => UiState {
                app_state: AppState::Menu,
                current: Some(UiStateTypes::Menu(UiMenuState {
                    selected_file_index,
                })),
            },
            AppState::Quiz => {
                if let Some(session) = &quiz_session {
                    // Comprehensive state tracking for all UI-changing elements
                    let quiz_state = UiQuizState {
                        current_index: session.current_index,
                        showing_answer: session.showing_answer,
                        ai_evaluation_in_progress: session.ai_evaluation_in_progress,
                        input_buffer_len: session.input_buffer.len(), // Text content length
                        cursor_position: session.cursor_position,     // Cursor position
                        input_scroll_y: session.input_scroll_y, // Scroll position for long text
                        has_ai_error: session.last_ai_error.is_some(), // Error message presence
                        questions_answered: session.questions_answered, // Progress indicator
                        ai_feedback_count: session
                            .flashcards
                            .iter()
                            .filter(|f| f.ai_feedback.is_some())
                            .count(), // AI feedback count
                    };
                    UiState {
                        app_state: AppState::Quiz,
                        current: Some(UiStateTypes::Quiz(quiz_state)),
                    }
                } else {
                    UiState {
                        app_state: AppState::Quiz,
                        current: None,
                    }
                }
            }
            AppState::QuizQuitConfirm => UiState {
                app_state: AppState::QuizQuitConfirm,
                current: None,
            },
            AppState::Summary => UiState {
                app_state: AppState::Summary,
                current: None,
            },
        };

        // Always draw on first iteration, then only redraw if state has changed
        let should_draw = is_first_draw || (current_ui_state != last_ui_state);

        if should_draw {
            terminal.draw(|f| match app_state {
                AppState::Menu => draw_menu(f, &csv_files, selected_file_index, ai_enabled),
                AppState::Quiz => {
                    if let Some(ref mut session) = quiz_session {
                        // Draw the quiz with current state (AI responses handled asynchronously)
                        draw_quiz(f, session, None);
                    }
                }
                AppState::QuizQuitConfirm => draw_quit_confirmation(f),
                AppState::Summary => {
                    if let Some(session) = &quiz_session {
                        draw_summary(f, session);
                    }
                }
            })?;
            last_ui_state = current_ui_state.clone();
            is_first_draw = false;
        }

        // Main event loop with tokio::select! for concurrent async operations
        tokio::select! {
            // Handle user input events
            Some(event_result) = event_stream.next() => {
                match event_result? {
                    Event::Key(key) => {
                        if key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            break;
                        }
                        match app_state {
                            AppState::Menu => match key.code {
                                KeyCode::Up => {
                                    selected_file_index = selected_file_index.saturating_sub(1);
                                }
                                KeyCode::Down => {
                                    if selected_file_index < csv_files.len().saturating_sub(1) {
                                        selected_file_index += 1;
                                    }
                                }
                                KeyCode::Enter => {
                                    if !csv_files.is_empty()
                                        && let Ok(flashcards) = load_csv(&csv_files[selected_file_index]) {
                                            let deck_name = csv_files[selected_file_index]
                                                .file_stem().map(|s| s.to_string_lossy().to_string())
                                                .unwrap_or_else(|| "unknown_deck".to_string());
                                            let mut cards = flashcards;
                                            cards.shuffle(&mut rand::thread_rng());

                                            let filename = get_quiz_filename(&deck_name);
                                            let mut output_file = match fs::File::create(&filename) {
                                                Ok(file) => file,
                                                Err(e) => {
                                                    eprintln!("Failed to create quiz file: {}", e);
                                                    return Ok(());
                                                }
                                            };

                                              let progress_header_position = write_session_header(
                                                  &mut output_file,
                                                  &deck_name,
                                                  cards.len(),
                                              )?;

                                              // Create async channels for this quiz session (buffered)
                                            let (request_tx, request_rx) = mpsc::channel::<AiRequest>(32);
                                            let (response_tx, response_rx) = mpsc::channel::<AiResponse>(32);

                                            // Spawn AI worker if enabled
                                            if ai_enabled {
                                                let _ai_handle = ai_worker::spawn_ai_worker(response_tx, request_rx);
                                            }

                                            let questions_total = cards.len();
                                            quiz_session = Some(QuizSession {
                                                flashcards: cards,
                                                current_index: 0,
                                                deck_name,
                                                showing_answer: false,
                                                input_buffer: String::new(),
                                                cursor_position: 0,
                                                output_file: Some(output_file),
                                                questions_total,
                                                questions_answered: 0,
                                                ai_enabled,
                                                ai_evaluation_in_progress: false,
                                                ai_last_evaluated_index: None,
                                                ai_evaluation_start_time: None,
                                                last_ai_error: None,
                                                ai_tx: if ai_enabled { Some(request_tx) } else { None },
                                                ai_rx: if ai_enabled { Some(response_rx) } else { None },
                                                progress_header_position,
                                                input_scroll_y: 0,
                                            });

                                            app_state = AppState::Quiz;
                                        }
                                }
                                KeyCode::Char('m') => {
                                    app_state = AppState::Menu;
                                    if let Some(mut session) = quiz_session.take()
                                        && let Some(file) = session.output_file.take() {
                                            drop(file);
                                        }
                                    quiz_session = None;
                                }
                                KeyCode::Esc => break,
                                _ => {}
                            },
                            AppState::Quiz => {
                                if let Some(session) = &mut quiz_session
                                    && let Err(e) = handle_quiz_input(session, key, &mut app_state) {
                                        eprintln!("Error writing to quiz file: {}", e);
                                    }
                            }
                            AppState::QuizQuitConfirm => match key.code {
                                KeyCode::Char('y') => {
                                    app_state = AppState::Menu;
                                    if let Some(mut session) = quiz_session.take()
                                        && let Some(file) = session.output_file.take() {
                                            drop(file);
                                        }
                                    quiz_session = None;
                                }
                                KeyCode::Char('n') => {
                                    app_state = AppState::Quiz;
                                }
                                _ => {}
                            },
                            AppState::Summary => match key.code {
                                KeyCode::Char('m') => {
                                    app_state = AppState::Menu;
                                    if let Some(mut session) = quiz_session.take()
                                        && let Some(file) = session.output_file.take() {
                                            drop(file);
                                        }
                                    quiz_session = None;
                                },
                                KeyCode::Esc => break,
                                _ => {}
                            },
                        }
                    },
                    Event::Paste(text) => {
                        if let AppState::Quiz = app_state
                            && let Some(session) = &mut quiz_session
                            && !session.showing_answer {
                            // Insert the entire pasted text at cursor position
                            for ch in text.chars() {
                                session.input_buffer.insert(session.cursor_position, ch);
                                session.cursor_position += 1;
                            }
                        }
                    }
                    Event::Mouse(mouse_event) => {
                        match mouse_event.kind {
                            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                                // Ignore scroll events to prevent navigation
                            }
                            _ => {}
                        }
                    }
                    // Force redraw by setting last_ui_state to an invalid state.
                    // NOTE: This does NOT affect what is actually drawn - the terminal.draw()
                    // closure uses the CURRENT app_state variable, not last_ui_state.
                    // Setting app_state to Menu here only makes current_ui_state != last_ui_state,
                    // triggering should_draw = true on the next loop iteration.
                    // The actual drawing uses the current app_state (Quiz, Menu, etc.).
                    Event::FocusGained => {
                        last_ui_state = UiState {
                            app_state: AppState::Menu,
                            current: None,
                        };
                    }
                    Event::FocusLost => {
                        // Do nothing - AI evaluation continues uninterrupted
                    }
                    Event::Resize(_, _) => {
                        last_ui_state = UiState {
                            app_state: AppState::Menu,
                            current: None,
                        };
                    }
                }
            },

            // Async AI response receiving
            Some(response) = async {
                if let Some(session) = &mut quiz_session {
                    if let Some(rx) = &mut session.ai_rx {
                        rx.recv().await
                    } else {
                        std::future::pending().await
                    }
                } else {
                    std::future::pending().await
                }
            } => {
                // Process the AI response immediately
                if let Some(mut session) = quiz_session.take() {
                    session.process_ai_responses(response);
                    quiz_session = Some(session);
                    // Force UI redraw for immediate AI feedback display
                    last_ui_state = UiState {
                        app_state: AppState::Menu,
                        current: None,
                    };
                }
            }

            // AI evaluation timeout checking (every 30 seconds)
            _ = ai_timeout_interval.tick() => {
                // Check for AI evaluation timeouts
                if let Some(mut session) = quiz_session.take() {
                    if session.ai_evaluation_in_progress
                        && let Some(start_time) = session.ai_evaluation_start_time
                            && start_time.elapsed() > Duration::from_secs(30) {
                                session.last_ai_error = Some(
                                    "AI evaluation timed out - press Ctrl+E to retry".to_string(),
                                );
                                session.ai_evaluation_in_progress = false;
                                logger::log("AI evaluation timed out after 30 seconds");

                                // Force UI redraw for timeout message
                                last_ui_state = UiState {
                                    app_state: AppState::Menu,
                                    current: None,
                                };
                            }
                    quiz_session = Some(session);
                }
            }
        }
    }

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableBracketedPaste,
        DisableMouseCapture,
        DisableFocusChange
    )?;
    terminal.show_cursor()?;

    Ok(())
}
