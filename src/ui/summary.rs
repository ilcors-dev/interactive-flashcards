use crate::models::QuizSession;
use crate::utils::truncate_string;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw_summary(f: &mut Frame, session: &QuizSession) {
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
            "m",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Main Menu  "),
        Span::styled(
            "Esc",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Quit  "),
    ])];
    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}
