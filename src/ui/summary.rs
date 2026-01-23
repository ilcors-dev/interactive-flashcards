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

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let mut summary_text = Text::default();
    summary_text.push_line(Line::from(format!(
        "Total: {}  |  Answered: {}  |  AI: {}",
        session.flashcards.len(),
        session.questions_answered,
        session
            .flashcards
            .iter()
            .filter(|c| c.ai_feedback.is_some())
            .count()
    )));
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
            truncate_string(&card.question, 50)
        )));
        if let Some(user_answer) = &card.user_answer {
            summary_text.push_line(Line::from(format!(
                "   Your: {}",
                truncate_string(user_answer, 46)
            )));
        }
        summary_text.push_line(Line::from(""));
    }

    let summary = Paragraph::new(summary_text)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Questions"));
    f.render_widget(summary, main_chunks[0]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(main_chunks[1]);

    let assessment_title = Paragraph::new("Session Assessment")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(assessment_title, right_chunks[0]);

    if session.assessment_loading {
        let loading_text = Paragraph::new("Analyzing session...")
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(loading_text, right_chunks[1]);
    } else if let Some(ref assessment) = session.session_assessment {
        let mut assessment_text = Text::default();

        let grade_color = if assessment.grade_percentage >= 70.0 {
            Color::Green
        } else if assessment.grade_percentage >= 40.0 {
            Color::Yellow
        } else {
            Color::Red
        };

        assessment_text.push_line(Line::from(vec![
            Span::styled("Grade: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{:.0}%", assessment.grade_percentage),
                Style::default()
                    .fg(grade_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
            Span::styled(&assessment.mastery_level, Style::default().fg(grade_color)),
        ]));
        assessment_text.push_line(Line::from(""));
        assessment_text.push_line(Line::from(vec![Span::styled(
            "Feedback:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]));
        assessment_text.push_line(Line::from(truncate_string(
            &assessment.overall_feedback,
            56,
        )));
        assessment_text.push_line(Line::from(""));

        if !assessment.strengths.is_empty() {
            assessment_text.push_line(Line::from(vec![Span::styled(
                "Strengths:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]));
            for strength in &assessment.strengths {
                assessment_text
                    .push_line(Line::from(format!("  ✓ {}", truncate_string(strength, 52))));
            }
            assessment_text.push_line(Line::from(""));
        }

        if !assessment.weaknesses.is_empty() {
            assessment_text.push_line(Line::from(vec![Span::styled(
                "Areas to Improve:",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )]));
            for weakness in &assessment.weaknesses {
                assessment_text
                    .push_line(Line::from(format!("  ✗ {}", truncate_string(weakness, 52))));
            }
            assessment_text.push_line(Line::from(""));
        }

        if !assessment.suggestions.is_empty() {
            assessment_text.push_line(Line::from(vec![Span::styled(
                "Suggestions:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]));
            for (i, suggestion) in assessment.suggestions.iter().enumerate() {
                assessment_text.push_line(Line::from(format!(
                    "  {}. {}",
                    i + 1,
                    truncate_string(suggestion, 52)
                )));
            }
        }

        let assessment_widget = Paragraph::new(assessment_text)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(assessment_widget, right_chunks[1]);

        if session.assessment_error.is_some() {
            let error_text = Paragraph::new("Analysis unavailable - [R]etry")
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(error_text, right_chunks[2]);
        } else {
            let help_text = Paragraph::new("[R]etry Analysis")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(help_text, right_chunks[2]);
        }
    } else if let Some(ref error) = session.assessment_error {
        let error_text = Paragraph::new(vec![
            Line::from("Analysis unavailable"),
            Line::from(""),
            Line::from(error.as_str()),
            Line::from(""),
            Line::from("[R]etry"),
        ])
        .style(Style::default().fg(Color::Red))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(error_text, right_chunks[1]);

        let help_text = Paragraph::new("[R]etry")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help_text, right_chunks[2]);
    } else {
        let no_assessment = Paragraph::new("No analysis available")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(no_assessment, right_chunks[1]);

        let help_text = Paragraph::new("")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help_text, right_chunks[2]);
    }

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
