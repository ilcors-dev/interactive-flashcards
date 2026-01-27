use crate::models::QuizSession;
use crate::ui::layout::calculate_summary_chunks;
use crate::utils::render_markdown;
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw_summary(f: &mut Frame, session: &QuizSession) {
    let layout = calculate_summary_chunks(f.area());

    let title_text = format!("Session Summary - {}", session.deck_name);
    let title = Paragraph::new(title_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, layout.header_area);

    // Assessment takes full width (no more horizontal split)
    let mut assessment_text = Text::default();

    // Add simplified stats header
    let ai_feedback_count = session
        .flashcards
        .iter()
        .filter(|c| c.ai_feedback.is_some())
        .count();

    // Calculate average score from AI feedback
    let avg_score = if ai_feedback_count > 0 {
        let total_score: f32 = session
            .flashcards
            .iter()
            .filter_map(|c| c.ai_feedback.as_ref())
            .map(|feedback| feedback.correctness_score)
            .sum();
        total_score / ai_feedback_count as f32
    } else {
        0.0
    };

    assessment_text.push_line(Line::from(format!(
        "Answered: {}  |  Avg Score: {:.0}%",
        session.questions_answered, avg_score
    )));
    assessment_text.push_line(Line::from(""));

    if session.assessment_loading {
        let loading_text = Paragraph::new("Analyzing session...")
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(loading_text, layout.assessment_content);
    } else if let Some(ref assessment) = session.session_assessment {
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
        let rendered_feedback = render_markdown(&assessment.overall_feedback);
        assessment_text.extend(rendered_feedback);
        assessment_text.push_line(Line::from(""));

        if !assessment.strengths.is_empty() {
            assessment_text.push_line(Line::from(vec![Span::styled(
                "Strengths:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]));
            for strength in &assessment.strengths {
                assessment_text.push_line(Line::from(format!("  ✓ {}", strength)));
            }
            assessment_text.push_line(Line::from(""));
        }

        if !assessment.weaknesses.is_empty() {
            assessment_text.push_line(Line::from(vec![Span::styled(
                "Areas to Improve:",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )]));
            for weakness in &assessment.weaknesses {
                assessment_text.push_line(Line::from(format!("  ✗ {}", weakness)));
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
                assessment_text.push_line(Line::from(format!("  {}. {}", i + 1, suggestion)));
            }
        }

        let assessment_widget = Paragraph::new(assessment_text)
            .wrap(Wrap { trim: true })
            .scroll((session.assessment_scroll_y, 0))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(assessment_widget, layout.assessment_content);

        if session.assessment_error.is_some() {
            let error_text = Paragraph::new("Analysis unavailable - [R]etry")
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(error_text, layout.assessment_help);
        } else {
            let help_text = Paragraph::new("[R]etry Analysis")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(help_text, layout.assessment_help);
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
        f.render_widget(error_text, layout.assessment_content);

        let help_text = Paragraph::new("[R]etry")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help_text, layout.assessment_help);
    } else {
        let no_assessment = Paragraph::new("No analysis available")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(no_assessment, layout.assessment_content);

        let help_text = Paragraph::new("")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help_text, layout.assessment_help);
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
    f.render_widget(help, layout.footer_area);
}
