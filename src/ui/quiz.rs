use crate::models::QuizSession;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw_quiz(f: &mut Frame, session: &mut QuizSession, ai_error: Option<&str>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Min(10),
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
        "Answer"
    } else {
        "Your Answer"
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

        // Add AI feedback, error, or loading in the same area
        if let Some(feedback) = &flashcard.ai_feedback {
            text.push_line(Line::from(""));
            text.push_line(Line::from(Span::styled(
                "AI Evaluation:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            text.push_line(Line::from(format!(
                "Score: {:.0}% - {}",
                feedback.correctness_score * 100.0,
                if feedback.is_correct {
                    "Correct"
                } else if feedback.correctness_score > 0.5 {
                    "Partially Correct"
                } else {
                    "Incorrect"
                }
            )));

            if !feedback.corrections.is_empty() {
                text.push_line(Line::from(""));
                text.push_line(Line::from("Corrections:"));
                for correction in &feedback.corrections {
                    text.push_line(Line::from(format!("• {}", correction)));
                }
            }

            text.push_line(Line::from(""));
            text.push_line(Line::from("Explanation:"));
            text.push_line(Line::from(feedback.explanation.as_str()));

            if !feedback.suggestions.is_empty() {
                text.push_line(Line::from(""));
                text.push_line(Line::from("Suggestions:"));
                for suggestion in &feedback.suggestions {
                    text.push_line(Line::from(format!("• {}", suggestion)));
                }
            }
        } else if let Some(error) = ai_error {
            text.push_line(Line::from(""));
            text.push_line(Line::from(error));
        } else if session.ai_enabled && session.ai_evaluation_in_progress {
            text.push_line(Line::from(""));
            text.push_line(Line::from("AI is evaluating your answer..."));
        }

        text
    } else {
        Text::from(if session.input_buffer.is_empty() {
            "[Type your answer here...]"
        } else {
            &session.input_buffer
        })
    };

    // Calculate scroll position for input mode to keep cursor visible
    let scroll_y = if !session.showing_answer {
        let visible_height = (chunks[2].height - 2) as usize; // Account for borders
        let text_width = (chunks[2].width - 2) as usize;
        let (cursor_line, _) = crate::calculate_wrapped_cursor_position(
            &session.input_buffer,
            session.cursor_position,
            text_width,
        );

        // Adjust scroll to keep cursor visible
        let mut new_scroll = session.input_scroll_y as usize;
        if cursor_line < new_scroll {
            new_scroll = cursor_line;
        } else if cursor_line >= new_scroll + visible_height {
            new_scroll = cursor_line - visible_height + 1;
        }
        session.input_scroll_y = new_scroll as u16;
        new_scroll as u16
    } else {
        0
    };

    let answer = Paragraph::new(answer_content)
        .wrap(Wrap { trim: true })
        .scroll((scroll_y, 0))
        .block(Block::default().borders(Borders::ALL).title(answer_title));
    f.render_widget(answer, chunks[2]);

    // Set cursor position when typing an answer
    if !session.showing_answer {
        // Calculate cursor position accounting for text wrapping
        let text_width = (chunks[2].width - 2) as usize; // Account for borders
        let (cursor_line, cursor_col) = crate::calculate_wrapped_cursor_position(
            &session.input_buffer,
            session.cursor_position,
            text_width,
        );
        let cursor_x = chunks[2].x + 1 + cursor_col as u16;
        let cursor_y = chunks[2].y + 1 + (cursor_line as u16).saturating_sub(scroll_y);
        f.set_cursor_position((cursor_x, cursor_y));
    }

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
            Span::styled(
                "Esc",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::from(" Quit to Menu  "),
        ]));
    }

    help_text.push(Line::from(vec![
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
        Span::from(" Next  "),
        Span::styled(
            "Esc",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Quit to Menu  "),
    ]));

    if session.ai_enabled {
        help_text.push(Line::from(vec![
            Span::styled(
                "Ctrl+E",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::from(" AI Re-evaluate  "),
            Span::styled(
                "Ctrl+X",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::from(" AI Cancel  "),
        ]));
    }

    help_text.push(Line::from(vec![
        Span::styled(
            "Ctrl+C",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Exit App"),
    ]));

    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[3]);
}

pub fn draw_quit_confirmation(f: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(5)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title = Paragraph::new("Quit to Menu")
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let message = Paragraph::new("Return to main menu?")
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(message, chunks[1]);

    let help_text = vec![Line::from(vec![
        Span::styled(
            "y",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Yes (Return to Menu)  "),
        Span::styled(
            "n",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::from(" No (Continue Quiz)  "),
        Span::styled(
            "Ctrl+C",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Exit App"),
    ])];
    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}
