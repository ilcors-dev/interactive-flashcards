use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::seq::SliceRandom;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::fs;
use std::io;

use std::time::UNIX_EPOCH;

use interactive_flashcards::{
    ai_worker, draw_menu, draw_quit_confirmation, draw_quiz, draw_summary, get_csv_files,
    handle_quiz_input, load_csv, logger, models, write_session_header, AppState, QuizSession,
};

fn get_quiz_filename(deck_name: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("quiz_{}_{}.txt", deck_name, timestamp)
}

fn main() -> io::Result<()> {
    logger::init();
    logger::log("Application started");

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app_state = AppState::Menu;
    let csv_files = get_csv_files();
    let mut selected_file_index: usize = 0;
    let mut quiz_session: Option<QuizSession> = None;
    let ai_enabled = std::env::var("OPENROUTER_API_KEY").is_ok();

    if ai_enabled {
        eprintln!("interactive-flashcards: AI evaluation enabled");
    }

    // Channels will be created when starting a quiz session

    loop {
        terminal.draw(|f| match app_state {
            AppState::Menu => draw_menu(f, &csv_files, selected_file_index, ai_enabled),
            AppState::Quiz => {
                if let Some(session) = &mut quiz_session {
                    // Check for AI evaluation timeout (30 seconds)
                    if session.ai_evaluation_in_progress
                        && let Some(start_time) = session.ai_evaluation_start_time
                            && start_time.elapsed() > std::time::Duration::from_secs(30) {
                                session.last_ai_error = Some(
                                    "AI evaluation timed out - press Ctrl+E to retry".to_string(),
                                );
                                session.ai_evaluation_in_progress = false;
                                logger::log("AI evaluation timed out after 30 seconds");
                            }

                    // Collect any pending AI responses first
                    let mut responses = Vec::new();
                    if let Some(ref ai_rx) = session.ai_rx {
                        while let Ok(response) = ai_rx.try_recv() {
                            responses.push(response);
                        }
                    }
                    // Process the collected responses
                    for response in responses {
                        session.process_ai_responses(response);
                    }
                    // Draw the quiz with current state
                    draw_quiz(f, session, session.last_ai_error.as_deref());
                }
            }
            AppState::QuizQuitConfirm => draw_quit_confirmation(f),
            AppState::Summary => {
                if let Some(session) = &quiz_session {
                    draw_summary(f, session);
                }
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
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
                                        .file_stem()
                                        .unwrap()
                                        .to_string_lossy()
                                        .to_string();
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

                                     // Create channels for this quiz session
                                    let (request_tx, request_rx) = std::sync::mpsc::channel::<models::AiRequest>();
                                    let (response_tx, response_rx) = std::sync::mpsc::channel::<models::AiResponse>();

                                    // Spawn AI worker if enabled
                                    if ai_enabled {
                                        let request_rx_clone = request_rx;
                                        let response_tx_clone = response_tx.clone();
                                        let _ = ai_worker::spawn_ai_worker(response_tx_clone, request_rx_clone);
                                    }

                                    quiz_session = Some(QuizSession {
                                        flashcards: cards.clone(),
                                        current_index: 0,
                                        deck_name,
                                        showing_answer: false,
                                        input_buffer: String::new(),
                                        cursor_position: 0,
                                        output_file: Some(output_file),
                                        questions_total: cards.len(),
                                        questions_answered: 0,
                                        ai_enabled,
                                        ai_evaluation_in_progress: false,
                                        ai_last_evaluated_index: None,
                                        ai_evaluation_start_time: None,
                                         ai_tx: if ai_enabled { Some(request_tx) } else { None },
                                         ai_rx: if ai_enabled { Some(response_rx) } else { None },
                                         last_ai_error: None,
                                         progress_header_position,
                                     });
                                    app_state = AppState::Quiz;
                                }
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
                            if let Some(file) = quiz_session.take() {
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
                        KeyCode::Char('r') => {
                            if let Some(session) = &mut quiz_session {
                                let mut cards = session.flashcards.clone();
                                for card in &mut cards {
                                    card.user_answer = None;
                                }
                                cards.shuffle(&mut rand::thread_rng());
                                session.flashcards = cards;
                                session.current_index = 0;
                                session.showing_answer = false;
                                session.input_buffer = String::new();
                                session.cursor_position = 0;
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
                }
            }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
