use crossterm::{
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture, Event, EventStream, KeyCode, KeyModifiers,
        MouseEventKind,
    },
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use interactive_flashcards::db::{self, flashcard, session};
use rand::seq::SliceRandom;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use futures::StreamExt;
use tokio::sync::mpsc;
use tokio::time::{self, Duration};

use interactive_flashcards::{
    ai_worker,
    db::session::SessionSummary,
    draw_menu, draw_quit_confirmation, draw_quiz, draw_summary, get_csv_files, handle_quiz_input,
    load_csv, logger,
    models::{
        AiRequest, AiResponse, AppState, Flashcard, QuizSession, UiMenuState, UiQuizState, UiState,
        UiStateTypes,
    },
};

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

    // Session history state - load at startup
    let mut sessions: Vec<SessionSummary> = Vec::new();
    let mut selected_session_index: usize = 0;
    let mut focused_panel: usize = 0; // 0 = CSV, 1 = Sessions
    let mut _delete_confirm: bool = false;

    // Load sessions at startup
    if let Ok(conn) = db::init_db() {
        sessions = session::list_sessions(&conn).unwrap_or_default();
    }

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
                    selected_session_index,
                    focused_panel,
                    sessions_count: sessions.len(),
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
                AppState::Menu => draw_menu(
                    f,
                    &csv_files,
                    selected_file_index,
                    &sessions,
                    selected_session_index,
                    focused_panel,
                    ai_enabled,
                ),
                AppState::Quiz => {
                    if let Some(ref mut session) = quiz_session {
                        // Draw the quiz with current state (AI responses handled asynchronously)
                        draw_quiz(f, session, None);
                    }
                }
                AppState::QuizQuitConfirm => draw_quit_confirmation(f),
                AppState::Summary => {
                    if let Some(ref mut session) = quiz_session {
                        draw_summary(f, session);
                        // Trigger session assessment if not already loading
                        if session.assessment_loading
                            && session.session_assessment.is_none()
                            && session.assessment_error.is_none()
                            && let Some(session_id) = session.session_id {
                                let deck_name = session.deck_name.clone();
                                let flashcards: Vec<_> = session
                                    .flashcards
                                    .iter()
                                    .map(|fc| {
                                        (
                                            fc.question.clone(),
                                            fc.answer.clone(),
                                            fc.user_answer.clone(),
                                            fc.ai_feedback.clone(),
                                        )
                                    })
                                    .collect();

                                if let Some(ref ai_tx) = session.ai_tx {
                                    let request = AiRequest::EvaluateSession {
                                        session_id,
                                        deck_name,
                                        flashcards,
                                    };
                                    let _ = ai_tx.try_send(request);
                                    logger::log("Triggered session assessment request");
                                } else if session.ai_enabled {
                                    // AI is enabled but no channel - create one
                                    let (request_tx, request_rx) = mpsc::channel::<AiRequest>(32);
                                    let (response_tx, response_rx) =
                                        mpsc::channel::<AiResponse>(32);
                                    let _ai_handle =
                                        ai_worker::spawn_ai_worker(response_tx, request_rx);

                                    let request = AiRequest::EvaluateSession {
                                        session_id,
                                        deck_name,
                                        flashcards,
                                    };
                                    let _ = request_tx.try_send(request);

                                    session.ai_tx = Some(request_tx);
                                    session.ai_rx = Some(response_rx);
                                    logger::log("Created new AI channel for session assessment");
                                }
                            }
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
                                KeyCode::Char('1') => {
                                    focused_panel = 0;
                                }
                                KeyCode::Char('2') => {
                                    focused_panel = 1;
                                }
                                KeyCode::Up => {
                                    if focused_panel == 0 {
                                        selected_file_index = selected_file_index.saturating_sub(1);
                                    } else {
                                        selected_session_index = selected_session_index.saturating_sub(1);
                                    }
                                }
                                KeyCode::Down => {
                                    if focused_panel == 0 {
                                        if selected_file_index < csv_files.len().saturating_sub(1) {
                                            selected_file_index += 1;
                                        }
                                    } else if !sessions.is_empty() && selected_session_index < sessions.len().saturating_sub(1) {
                                        selected_session_index += 1;
                                    }
                                }
                                KeyCode::Enter => {
                                    if focused_panel == 0 {
                                        // CSV panel - start new quiz
                                        if !csv_files.is_empty()
                                            && let Ok(flashcards) = load_csv(&csv_files[selected_file_index]) {
                                            let deck_name = csv_files[selected_file_index]
                                                .file_stem().map(|s| s.to_string_lossy().to_string())
                                                .unwrap_or_else(|| "unknown_deck".to_string());
                                            let mut cards = flashcards;
                                            cards.shuffle(&mut rand::thread_rng());

                                            let conn = match db::init_db() {
                                                Ok(conn) => conn,
                                                Err(e) => {
                                                    eprintln!("Failed to initialize database: {}", e);
                                                    return Ok(());
                                                }
                                            };

                                            let session_id = match session::create_session(&conn, &deck_name, cards.len()) {
                                                Ok(id) => id,
                                                Err(e) => {
                                                    eprintln!("Failed to create session: {}", e);
                                                    return Ok(());
                                                }
                                            };

                                            let flashcards_data: Vec<(String, String)> = cards.iter()
                                                .map(|c| (c.question.clone(), c.answer.clone()))
                                                .collect();

                                            if let Err(e) = flashcard::initialize_flashcards(&conn, session_id, &flashcards_data) {
                                                eprintln!("Failed to initialize flashcards: {}", e);
                                                return Ok(());
                                            }

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
                                                session_id: Some(session_id),
                                                questions_total,
                                                questions_answered: 0,
                                                ai_enabled,
                                                ai_evaluation_in_progress: false,
                                                ai_last_evaluated_index: None,
                                                ai_evaluation_start_time: None,
                                                last_ai_error: None,
                                                ai_tx: if ai_enabled { Some(request_tx) } else { None },
                                                ai_rx: if ai_enabled { Some(response_rx) } else { None },
                                                input_scroll_y: 0,
                                                session_assessment: None,
                                                assessment_loading: false,
                                                assessment_error: None,
                                            });

                                            app_state = AppState::Quiz;
                                        }
                                    } else {
                                        // Sessions panel - resume session
                                        if !sessions.is_empty() && selected_session_index < sessions.len() {
                                            let session_id = sessions[selected_session_index].id;
                                            if let Ok(conn) = db::init_db()
                                                 && let Ok(Some((session_data, flashcards_data))) = session::get_session_detail(&conn, session_id) {
                                                let cards: Vec<Flashcard> = flashcards_data
                                                    .into_iter()
                                                    .map(|fc| Flashcard {
                                                        question: fc.question,
                                                        answer: fc.answer,
                                                        user_answer: fc.user_answer,
                                                        ai_feedback: fc.ai_feedback,
                                                        written_to_file: true,
                                                    })
                                                    .collect();

                                                let mut resume_index = 0;
                                                let mut showing_answer = false;
                                                let mut input_buffer = String::new();
                                                let mut cursor_position = 0;
                                                for (i, card) in cards.iter().enumerate() {
                                                    if card.user_answer.is_none() {
                                                        resume_index = i;
                                                        break;
                                                    }
                                                    resume_index = i;
                                                }
                                                if resume_index < cards.len() {
                                                    showing_answer = cards[resume_index].user_answer.is_some();
                                                    if showing_answer {
                                                        input_buffer = cards[resume_index].user_answer.clone().unwrap_or_default();
                                                        cursor_position = input_buffer.len();
                                                    }
                                                }

                                                let (request_tx, request_rx) = mpsc::channel::<AiRequest>(32);
                                                let (response_tx, response_rx) = mpsc::channel::<AiResponse>(32);

                                                if ai_enabled {
                                                    let _ai_handle = ai_worker::spawn_ai_worker(response_tx, request_rx);
                                                }

                                                quiz_session = Some(QuizSession {
                                                    flashcards: cards,
                                                    current_index: resume_index,
                                                    deck_name: session_data.deck_name,
                                                    showing_answer,
                                                    input_buffer,
                                                    cursor_position,
                                                    session_id: Some(session_id),
                                                    questions_total: session_data.questions_total,
                                                    questions_answered: session_data.questions_answered,
                                                    ai_enabled,
                                                    ai_evaluation_in_progress: false,
                                                    ai_last_evaluated_index: None,
                                                    ai_evaluation_start_time: None,
                                                    last_ai_error: None,
                                                    ai_tx: if ai_enabled { Some(request_tx) } else { None },
                                                    ai_rx: if ai_enabled { Some(response_rx) } else { None },
                                                    input_scroll_y: 0,
                                                    session_assessment: None,
                                                    assessment_loading: false,
                                                    assessment_error: None,
                                                });

                                                app_state = AppState::Quiz;
                                            }
                                        }
                                    }
                                }
                                KeyCode::Esc => break,
                                _ => {}
                            },
                            AppState::Quiz => {
                                if let Some(session) = &mut quiz_session
                                    && let Err(e) = handle_quiz_input(session, key, &mut app_state) {
                                        eprintln!("Error handling quiz input: {}", e);
                                    }
                            }
                            AppState::QuizQuitConfirm => match key.code {
                                KeyCode::Char('y') => {
                                    app_state = AppState::Menu;
                                    quiz_session = None;
                                    // Refresh sessions list
                                    if let Ok(conn) = db::init_db() {
                                        sessions = session::list_sessions(&conn).unwrap_or_default();
                                    }
                                }
                                KeyCode::Char('n') => {
                                    app_state = AppState::Quiz;
                                }
                                _ => {}
                            },
                            AppState::Summary => match key.code {
                                KeyCode::Char('m') => {
                                    app_state = AppState::Menu;
                                    quiz_session = None;
                                    // Refresh sessions list
                                    if let Ok(conn) = db::init_db() {
                                        sessions = session::list_sessions(&conn).unwrap_or_default();
                                    }
                                },
                                KeyCode::Char('r') | KeyCode::Char('R') => {
                                    if let Some(ref mut session) = quiz_session
                                        && (session.session_assessment.is_none() || session.assessment_error.is_some()) {
                                            // Retry assessment
                                            session.assessment_loading = true;
                                            session.assessment_error = None;

                                            if let Some(session_id) = session.session_id {
                                                let deck_name = session.deck_name.clone();
                                                let flashcards: Vec<_> = session.flashcards.iter().map(|fc| {
                                                    (
                                                        fc.question.clone(),
                                                        fc.answer.clone(),
                                                        fc.user_answer.clone(),
                                                        fc.ai_feedback.clone(),
                                                    )
                                                }).collect();

                                                if let Some(ref ai_tx) = session.ai_tx {
                                                    let request = AiRequest::EvaluateSession {
                                                        session_id,
                                                        deck_name,
                                                        flashcards,
                                                    };
                                                    let _ = ai_tx.try_send(request);
                                                } else if session.ai_enabled {
                                                    // Create new channel if needed
                                                    let (request_tx, request_rx) = mpsc::channel::<AiRequest>(32);
                                                    let (response_tx, response_rx) = mpsc::channel::<AiResponse>(32);
                                                    let _ai_handle = ai_worker::spawn_ai_worker(response_tx, request_rx);

                                                    let request = AiRequest::EvaluateSession {
                                                        session_id,
                                                        deck_name,
                                                        flashcards,
                                                    };
                                                    let _ = request_tx.try_send(request);

                                                    session.ai_tx = Some(request_tx);
                                                    session.ai_rx = Some(response_rx);
                                                }
                                            }
                                        }
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
