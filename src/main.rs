use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::seq::SliceRandom;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Terminal,
};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct Flashcard {
    question: String,
    answer: String,
    user_answer: Option<String>,
}

#[derive(Debug)]
struct QuizSession {
    flashcards: Vec<Flashcard>,
    current_index: usize,
    deck_name: String,
    showing_answer: bool,
    input_buffer: String,
}

#[derive(Debug, PartialEq)]
enum AppState {
    Menu,
    Quiz,
    Summary,
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app_state = AppState::Menu;
    let csv_files = get_csv_files();
    let mut selected_file_index: usize = 0;
    let mut quiz_session: Option<QuizSession> = None;

    loop {
        terminal.draw(|f| match app_state {
            AppState::Menu => draw_menu(f, &csv_files, selected_file_index),
            AppState::Quiz => {
                if let Some(session) = &quiz_session {
                    draw_quiz(f, session);
                }
            }
            AppState::Summary => {
                if let Some(session) = &quiz_session {
                    draw_summary(f, session);
                }
            }
        })?;

        if let Event::Key(key) = event::read()? {
            match app_state {
                AppState::Menu => match key.code {
                    KeyCode::Up => {
                        if selected_file_index > 0 {
                            selected_file_index -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if selected_file_index < csv_files.len().saturating_sub(1) {
                            selected_file_index += 1;
                        }
                    }
                    KeyCode::Enter => {
                        if !csv_files.is_empty() {
                            if let Ok(flashcards) = load_csv(&csv_files[selected_file_index]) {
                                let deck_name = csv_files[selected_file_index]
                                    .file_stem()
                                    .unwrap()
                                    .to_string_lossy()
                                    .to_string();
                                let mut cards = flashcards;
                                cards.shuffle(&mut rand::thread_rng());
                                quiz_session = Some(QuizSession {
                                    flashcards: cards,
                                    current_index: 0,
                                    deck_name,
                                    showing_answer: false,
                                    input_buffer: String::new(),
                                });
                                app_state = AppState::Quiz;
                            }
                        }
                    }
                    KeyCode::Char('q') => break,
                    _ => {}
                },
                AppState::Quiz => {
                    if let Some(session) = &mut quiz_session {
                        handle_quiz_input(session, key.code, &mut app_state);
                    }
                }
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
                            app_state = AppState::Quiz;
                        }
                    }
                    KeyCode::Char('m') => {
                        app_state = AppState::Menu;
                        quiz_session = None;
                    }
                    KeyCode::Char('q') => break,
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

fn get_csv_files() -> Vec<PathBuf> {
    let flashcards_dir = PathBuf::from("flashcards");
    let mut files = Vec::new();

    if flashcards_dir.exists() && flashcards_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&flashcards_dir) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "csv" {
                        files.push(entry.path());
                    }
                }
            }
        }
    }

    files.sort();
    files
}

fn load_csv(path: &PathBuf) -> io::Result<Vec<Flashcard>> {
    let content = fs::read_to_string(path)?;
    let mut flashcards = Vec::new();

    for line in content.lines() {
        if let Some((question, answer)) = parse_csv_line(line) {
            if !question.trim().is_empty() && !answer.trim().is_empty() {
                flashcards.push(Flashcard {
                    question,
                    answer,
                    user_answer: None,
                });
            }
        }
    }

    Ok(flashcards)
}

fn parse_csv_line(line: &str) -> Option<(String, String)> {
    let mut chars = line.chars().peekable();
    let mut question = String::new();
    let mut answer = String::new();
    let mut current_field = &mut question;
    let mut in_quotes = false;
    let mut field_index = 0;

    while let Some(c) = chars.next() {
        match c {
            '"' if !in_quotes => {
                in_quotes = true;
            }
            '"' if in_quotes => {
                if chars.peek() == Some(&',') {
                    chars.next();
                    in_quotes = false;
                    if field_index == 0 {
                        current_field = &mut answer;
                        field_index = 1;
                    }
                } else if chars.peek() == Some(&'"') {
                    chars.next();
                    current_field.push('"');
                } else {
                    in_quotes = false;
                    if field_index == 0 {
                        current_field = &mut answer;
                        field_index = 1;
                    }
                }
            }
            ',' if !in_quotes && field_index == 0 => {
                field_index = 1;
                current_field = &mut answer;
            }
            _ => {
                current_field.push(c);
            }
        }
    }

    Some((question, answer))
}

fn draw_menu(f: &mut ratatui::Frame, csv_files: &[PathBuf], selected_index: usize) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title = Paragraph::new("📚 Interactive Flashcards")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let items: Vec<ListItem> = csv_files
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            let style = if i == selected_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(name).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Select a Deck"),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_widget(list, chunks[1]);

    let help_text = vec![Line::from(vec![
        Span::styled(
            "↑/↓",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Navigate  "),
        Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Select  "),
        Span::styled(
            "q",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Quit"),
    ])];
    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}

fn draw_quiz(f: &mut ratatui::Frame, session: &QuizSession) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(f.area());

    let flashcard = &session.flashcards[session.current_index];
    let progress = format!(
        "Question {} / {} - {}",
        session.current_index + 1,
        session.flashcards.len(),
        session.deck_name
    );

    let header = Paragraph::new(progress)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    let question_text = Text::from(flashcard.question.as_str());
    let question = Paragraph::new(question_text)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Question"));
    f.render_widget(question, chunks[1]);

    let answer_title = if session.showing_answer {
        "Answer (Press Enter to continue)"
    } else {
        "Your Answer (Press Enter to submit)"
    };

    let answer_content = if session.showing_answer {
        let mut text = Text::default();
        text.push_line(Line::from(Span::styled(
            "Correct Answer:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));
        text.push_line(Line::from(""));
        text.push_line(Line::from(flashcard.answer.as_str()));
        if let Some(user_answer) = &flashcard.user_answer {
            text.push_line(Line::from(""));
            text.push_line(Line::from(Span::styled(
                "Your Answer:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            text.push_line(Line::from(user_answer.as_str()));
        }
        text
    } else {
        Text::from(if session.input_buffer.is_empty() {
            "[Type your answer here...]"
        } else {
            &session.input_buffer
        })
    };

    let answer = Paragraph::new(answer_content)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title(answer_title));
    f.render_widget(answer, chunks[2]);

    let mut help_text = Vec::new();
    if !session.showing_answer {
        help_text.push(Line::from(vec![
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::from(" Submit Answer  "),
        ]));
    }
    help_text.push(Line::from(vec![
        Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Next  "),
        Span::styled(
            "↑/↓",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Previous  "),
        Span::styled(
            "m",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Menu"),
    ]));

    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[3]);
}

fn handle_quiz_input(session: &mut QuizSession, key: KeyCode, app_state: &mut AppState) {
    if !session.showing_answer {
        match key {
            KeyCode::Char('m') => {
                *app_state = AppState::Menu;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if session.current_index < session.flashcards.len().saturating_sub(1) {
                    session.current_index += 1;
                    session.showing_answer = false;
                    session.input_buffer = session.flashcards[session.current_index]
                        .user_answer
                        .as_ref()
                        .unwrap_or(&String::new())
                        .clone();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if session.current_index > 0 {
                    session.current_index -= 1;
                    session.showing_answer = false;
                    session.input_buffer = session.flashcards[session.current_index]
                        .user_answer
                        .as_ref()
                        .unwrap_or(&String::new())
                        .clone();
                }
            }
            KeyCode::Enter => {
                if !session.input_buffer.trim().is_empty() {
                    session.flashcards[session.current_index].user_answer =
                        Some(session.input_buffer.clone());
                }
                session.input_buffer.clear();
                session.showing_answer = true;
            }
            KeyCode::Backspace => {
                session.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                session.input_buffer.push(c);
            }
            _ => {}
        }
    } else {
        match key {
            KeyCode::Char('m') => {
                *app_state = AppState::Menu;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if session.current_index < session.flashcards.len().saturating_sub(1) {
                    session.current_index += 1;
                    session.showing_answer = false;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if session.current_index > 0 {
                    session.current_index -= 1;
                    session.showing_answer = false;
                }
            }
            KeyCode::Enter => {
                if session.current_index < session.flashcards.len().saturating_sub(1) {
                    session.current_index += 1;
                    session.showing_answer = false;
                } else {
                    *app_state = AppState::Summary;
                }
            }
            _ => {}
        }
    }
}

fn draw_summary(f: &mut ratatui::Frame, session: &QuizSession) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title_text = format!("Session Summary - {}", session.deck_name);
    let title = Paragraph::new(title_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let mut summary_text = Text::default();
    summary_text.push_line(Line::from(format!(
        "Total Questions: {}",
        session.flashcards.len()
    )));
    summary_text.push_line(Line::from(""));
    summary_text.push_line(Line::from("Answers:"));
    summary_text.push_line(Line::from(""));

    for (i, card) in session.flashcards.iter().enumerate() {
        let answered = if card.user_answer.is_some() {
            "[✓]"
        } else {
            "[ ]"
        };
        summary_text.push_line(Line::from(format!(
            "{} {}. {}",
            answered,
            i + 1,
            truncate_string(&card.question, 60)
        )));
        if let Some(user_answer) = &card.user_answer {
            summary_text.push_line(Line::from(format!(
                "   Your Answer: {}",
                truncate_string(user_answer, 56)
            )));
        }
        summary_text.push_line(Line::from(""));
    }

    let summary = Paragraph::new(summary_text)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(summary, chunks[1]);

    let help_text = vec![Line::from(vec![
        Span::styled(
            "r",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Restart Deck  "),
        Span::styled(
            "m",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Main Menu  "),
        Span::styled(
            "q",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Quit"),
    ])];
    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv_simple() {
        let line = "What is 2+2?,Four";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "What is 2+2?");
        assert_eq!(answer, "Four");
    }

    #[test]
    fn test_parse_csv_with_quotes() {
        let line = "\"What is 2+2?\",\"Four\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "What is 2+2?");
        assert_eq!(answer, "Four");
    }

    #[test]
    fn test_parse_csv_with_commas_in_answer() {
        let line = "\"What is 2+2?\",\"Four, or 4\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "What is 2+2?");
        assert_eq!(answer, "Four, or 4");
    }

    #[test]
    fn test_parse_csv_with_commas_in_question() {
        let line = "\"What is 2+2, 3+3?\",\"Four\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "What is 2+2, 3+3?");
        assert_eq!(answer, "Four");
    }

    #[test]
    fn test_parse_csv_with_escaped_quotes() {
        let line = "\"What is \"\"quoted\"\"?\",\"Answer with \"\"quotes\"\"\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "What is \"quoted\"?");
        assert_eq!(answer, "Answer with \"quotes\"");
    }

    #[test]
    fn test_parse_csv_empty_fields() {
        let line = ",";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "");
        assert_eq!(answer, "");
    }

    #[test]
    fn test_parse_csv_complex_example() {
        let line = "\"In a CSV, what does a comma do?\",\"It separates fields, but can be part of a field if quoted\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "In a CSV, what does a comma do?");
        assert_eq!(
            answer,
            "It separates fields, but can be part of a field if quoted"
        );
    }

    #[test]
    fn test_parse_csv_only_question_quoted() {
        let line = "\"What is 2+2?\",Four";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "What is 2+2?");
        assert_eq!(answer, "Four");
    }

    #[test]
    fn test_parse_csv_only_answer_quoted() {
        let line = "What is 2+2?,\"Four\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "What is 2+2?");
        assert_eq!(answer, "Four");
    }

    #[test]
    fn test_flashcard_creation() {
        let card = Flashcard {
            question: "Question?".to_string(),
            answer: "Answer".to_string(),
            user_answer: None,
        };
        assert_eq!(card.question, "Question?");
        assert_eq!(card.answer, "Answer");
        assert!(card.user_answer.is_none());
    }

    #[test]
    fn test_flashcard_with_user_answer() {
        let card = Flashcard {
            question: "Question?".to_string(),
            answer: "Answer".to_string(),
            user_answer: Some("My Answer".to_string()),
        };
        assert_eq!(card.question, "Question?");
        assert_eq!(card.answer, "Answer");
        assert!(card.user_answer.is_some());
        assert_eq!(card.user_answer.unwrap(), "My Answer");
    }

    #[test]
    fn test_quiz_session_creation() {
        let cards = vec![
            Flashcard {
                question: "Q1".to_string(),
                answer: "A1".to_string(),
                user_answer: None,
            },
            Flashcard {
                question: "Q2".to_string(),
                answer: "A2".to_string(),
                user_answer: None,
            },
        ];
        let session = QuizSession {
            flashcards: cards,
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
        };
        assert_eq!(session.flashcards.len(), 2);
        assert_eq!(session.current_index, 0);
        assert_eq!(session.deck_name, "Test");
        assert!(!session.showing_answer);
        assert!(session.input_buffer.is_empty());
    }

    #[test]
    fn test_truncate_string_no_truncation() {
        let s = "Short string";
        let result = truncate_string(s, 20);
        assert_eq!(result, "Short string");
    }

    #[test]
    fn test_truncate_string_with_truncation() {
        let s = "This is a very long string that should be truncated";
        let result = truncate_string(s, 20);
        assert_eq!(result, "This is a very lo...");
        assert!(result.len() <= 20);
    }

    #[test]
    fn test_truncate_string_exact_length() {
        let s = "Exactly twenty!!";
        let result = truncate_string(s, 20);
        assert_eq!(result, "Exactly twenty!!");
    }

    #[test]
    fn test_truncate_string_empty() {
        let s = "";
        let result = truncate_string(s, 20);
        assert_eq!(result, "");
    }

    #[test]
    fn test_parse_csv_line_with_newlines_in_quoted_field() {
        let line = "\"Question\",\"Answer with, comma\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "Question");
        assert_eq!(answer, "Answer with, comma");
    }

    #[test]
    fn test_flashcard_clone() {
        let card = Flashcard {
            question: "Q".to_string(),
            answer: "A".to_string(),
            user_answer: Some("UA".to_string()),
        };
        let cloned = card.clone();
        assert_eq!(card.question, cloned.question);
        assert_eq!(card.answer, cloned.answer);
        assert_eq!(card.user_answer, cloned.user_answer);
    }

    #[test]
    fn test_input_buffer_operations() {
        let mut buffer = String::new();
        buffer.push('H');
        buffer.push('i');
        assert_eq!(buffer, "Hi");
        buffer.pop();
        assert_eq!(buffer, "H");
        assert!(buffer.trim().is_empty() == false);
    }

    #[test]
    fn test_quiz_session_state_transitions() {
        let cards = vec![Flashcard {
            question: "Q1".to_string(),
            answer: "A1".to_string(),
            user_answer: None,
        }];
        let mut session = QuizSession {
            flashcards: cards.clone(),
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
        };

        session.showing_answer = true;
        assert!(session.showing_answer);

        session.flashcards[0].user_answer = Some("My Answer".to_string());
        assert!(session.flashcards[0].user_answer.is_some());
    }

    #[test]
    fn test_multiple_flashcards_navigation() {
        let cards = vec![
            Flashcard {
                question: "Q1".to_string(),
                answer: "A1".to_string(),
                user_answer: None,
            },
            Flashcard {
                question: "Q2".to_string(),
                answer: "A2".to_string(),
                user_answer: None,
            },
            Flashcard {
                question: "Q3".to_string(),
                answer: "A3".to_string(),
                user_answer: None,
            },
        ];
        let mut session = QuizSession {
            flashcards: cards,
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
        };

        assert_eq!(session.current_index, 0);
        session.current_index += 1;
        assert_eq!(session.current_index, 1);
        session.current_index += 1;
        assert_eq!(session.current_index, 2);
        session.current_index -= 1;
        assert_eq!(session.current_index, 1);
    }

    #[test]
    fn test_state_preservation_on_navigation() {
        let mut cards = vec![
            Flashcard {
                question: "Q1".to_string(),
                answer: "A1".to_string(),
                user_answer: None,
            },
            Flashcard {
                question: "Q2".to_string(),
                answer: "A2".to_string(),
                user_answer: None,
            },
        ];

        cards[0].user_answer = Some("Answer 1".to_string());
        cards[1].user_answer = Some("Answer 2".to_string());

        let session = QuizSession {
            flashcards: cards,
            current_index: 1,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
        };

        assert_eq!(
            session.flashcards[0].user_answer,
            Some("Answer 1".to_string())
        );
        assert_eq!(
            session.flashcards[1].user_answer,
            Some("Answer 2".to_string())
        );
    }

    #[test]
    fn test_empty_answer_submission() {
        let mut buffer = String::new();
        assert!(buffer.trim().is_empty());
        buffer.push(' ');
        assert!(buffer.trim().is_empty());
        buffer.push('A');
        assert!(!buffer.trim().is_empty());
    }

    #[test]
    fn test_saturating_sub_index_bounds() {
        let cards = vec![Flashcard {
            question: "Q1".to_string(),
            answer: "A1".to_string(),
            user_answer: None,
        }];
        let session = QuizSession {
            flashcards: cards,
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
        };

        let new_index = session.current_index.saturating_sub(1);
        assert_eq!(new_index, 0);

        let max_index = session.flashcards.len().saturating_sub(1);
        assert_eq!(max_index, 0);
    }

    #[test]
    fn test_load_csv_with_empty_lines() {
        let content = "Q1,A1\n\nQ2,A2\n\nQ3,A3";
        let mut flashcards = Vec::new();

        for line in content.lines() {
            if let Some((question, answer)) = parse_csv_line(line) {
                if !question.trim().is_empty() && !answer.trim().is_empty() {
                    flashcards.push(Flashcard {
                        question,
                        answer,
                        user_answer: None,
                    });
                }
            }
        }

        assert_eq!(flashcards.len(), 3);
        assert_eq!(flashcards[0].question, "Q1");
        assert_eq!(flashcards[1].question, "Q2");
        assert_eq!(flashcards[2].question, "Q3");
    }

    #[test]
    fn test_load_csv_filters_empty_fields() {
        let content = "Q1,A1\n,A2\nQ2,\n,Q3\n";
        let mut flashcards = Vec::new();

        for line in content.lines() {
            if let Some((question, answer)) = parse_csv_line(line) {
                if !question.trim().is_empty() && !answer.trim().is_empty() {
                    flashcards.push(Flashcard {
                        question,
                        answer,
                        user_answer: None,
                    });
                }
            }
        }

        assert_eq!(flashcards.len(), 1);
        assert_eq!(flashcards[0].question, "Q1");
        assert_eq!(flashcards[0].answer, "A1");
    }

    #[test]
    fn test_answer_restoration_on_navigation() {
        let mut cards = vec![
            Flashcard {
                question: "Q1".to_string(),
                answer: "A1".to_string(),
                user_answer: None,
            },
            Flashcard {
                question: "Q2".to_string(),
                answer: "A2".to_string(),
                user_answer: None,
            },
        ];

        cards[0].user_answer = Some("My Answer 1".to_string());

        let mut session = QuizSession {
            flashcards: cards,
            current_index: 1,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
        };

        session.current_index = 0;
        session.showing_answer = false;
        session.input_buffer = session.flashcards[session.current_index]
            .user_answer
            .as_ref()
            .unwrap_or(&String::new())
            .clone();

        assert_eq!(session.input_buffer, "My Answer 1");
    }

    #[test]
    fn test_no_answer_restoration_when_none() {
        let cards = vec![Flashcard {
            question: "Q1".to_string(),
            answer: "A1".to_string(),
            user_answer: None,
        }];

        let mut session = QuizSession {
            flashcards: cards,
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
        };

        session.input_buffer = session.flashcards[session.current_index]
            .user_answer
            .as_ref()
            .unwrap_or(&String::new())
            .clone();

        assert!(session.input_buffer.is_empty());
    }

    #[test]
    fn test_answer_submission_non_empty() {
        let mut session = QuizSession {
            flashcards: vec![Flashcard {
                question: "Q1".to_string(),
                answer: "A1".to_string(),
                user_answer: None,
            }],
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::from("My Answer"),
        };

        if !session.input_buffer.trim().is_empty() {
            session.flashcards[session.current_index].user_answer =
                Some(session.input_buffer.clone());
        }

        assert_eq!(
            session.flashcards[session.current_index].user_answer,
            Some("My Answer".to_string())
        );
    }

    #[test]
    fn test_answer_submission_empty() {
        let mut session = QuizSession {
            flashcards: vec![Flashcard {
                question: "Q1".to_string(),
                answer: "A1".to_string(),
                user_answer: None,
            }],
            current_index: 0,
            deck_name: "Test".to_string(),
            showing_answer: false,
            input_buffer: String::from("   "),
        };

        if !session.input_buffer.trim().is_empty() {
            session.flashcards[session.current_index].user_answer =
                Some(session.input_buffer.clone());
        }

        assert!(session.flashcards[session.current_index]
            .user_answer
            .is_none());
    }

    #[test]
    fn test_app_state_transitions() {
        let mut state = AppState::Menu;
        assert_eq!(state, AppState::Menu);

        state = AppState::Quiz;
        assert_eq!(state, AppState::Quiz);

        state = AppState::Summary;
        assert_eq!(state, AppState::Summary);

        state = AppState::Menu;
        assert_eq!(state, AppState::Menu);
    }

    #[test]
    fn test_parse_csv_real_world_example() {
        let line = "\"What is the defining characteristic of a MANET?\",\"It is an infrastructure-less network where all nodes are potentially mobile and communicate directly with each other.\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "What is the defining characteristic of a MANET?");
        assert_eq!(
            answer,
            "It is an infrastructure-less network where all nodes are potentially mobile and communicate directly with each other."
        );
    }

    #[test]
    fn test_parse_csv_multiple_quotes() {
        let line = "\"Is \"\"quoted\"\" text supported?\",\"Yes, \"\"it works\"\" correctly\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "Is \"quoted\" text supported?");
        assert_eq!(answer, "Yes, \"it works\" correctly");
    }

    #[test]
    fn test_input_buffer_character_addition() {
        let mut buffer = String::new();
        buffer.push('H');
        buffer.push('e');
        buffer.push('l');
        buffer.push('l');
        buffer.push('o');
        assert_eq!(buffer, "Hello");
        buffer.push(' ');
        buffer.push('W');
        buffer.push('o');
        buffer.push('r');
        buffer.push('l');
        buffer.push('d');
        assert_eq!(buffer, "Hello World");
    }

    #[test]
    fn test_input_buffer_backspace() {
        let mut buffer = String::from("Hello");
        buffer.pop();
        assert_eq!(buffer, "Hell");
        buffer.pop();
        buffer.pop();
        assert_eq!(buffer, "He");
        buffer.pop();
        buffer.pop();
        assert!(buffer.is_empty());
        buffer.pop();
        assert!(buffer.is_empty());
    }
}
