use crate::models::QuizSession;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw_quiz(f: &mut Frame, session: &QuizSession) {
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
