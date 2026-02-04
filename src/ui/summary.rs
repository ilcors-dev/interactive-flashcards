use crate::models::QuizSession;
use crate::ui::layout::calculate_summary_chunks;
use crate::utils::{calculate_max_scroll, estimate_text_height, render_markdown};
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw_summary(f: &mut Frame, session: &mut QuizSession) {
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

    let mut content_text = Text::default();

    // Calculate stats from AI feedback scores
    let (answered_count, avg_score) = session.calculate_stats();

    if session.assessment_loading {
        content_text.push_line(Line::from(vec![
            Span::raw("Answered: "),
            Span::styled(
                format!("{}/{}", answered_count, session.questions_total),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]));
        content_text.push_line(Line::from(""));
        content_text.push_line(Line::from(Span::styled(
            "Analyzing session...",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
    } else if let Some(ref assessment) = session.session_assessment {
        let grade_color = if assessment.grade_percentage >= 70.0 {
            Color::Green
        } else if assessment.grade_percentage >= 40.0 {
            Color::Yellow
        } else {
            Color::Red
        };

        // Show answered count and AI grade on first line
        content_text.push_line(Line::from(vec![
            Span::raw("Answered: "),
            Span::styled(
                format!("{}/{}", answered_count, session.questions_total),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  "),
            Span::styled(
                format!("{:.0}%", assessment.grade_percentage),
                Style::default()
                    .fg(grade_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(&assessment.mastery_level, Style::default().fg(grade_color)),
        ]));
        content_text.push_line(Line::from(""));

        content_text.push_line(Line::from(vec![Span::styled(
            "Feedback:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]));
        let rendered_feedback = render_markdown(&assessment.overall_feedback);
        content_text.extend(rendered_feedback);
        content_text.push_line(Line::from(""));

        if !assessment.strengths.is_empty() {
            content_text.push_line(Line::from(vec![Span::styled(
                "Strengths:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]));
            for strength in &assessment.strengths {
                let rendered = render_markdown(&format!("  ✓ {}", strength));
                content_text.extend(rendered);
            }
            content_text.push_line(Line::from(""));
        }

        if !assessment.weaknesses.is_empty() {
            content_text.push_line(Line::from(vec![Span::styled(
                "Areas to Improve:",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )]));
            for weakness in &assessment.weaknesses {
                let rendered = render_markdown(&format!("  ✗ {}", weakness));
                content_text.extend(rendered);
            }
            content_text.push_line(Line::from(""));
        }

        if !assessment.misconceptions.is_empty() {
            content_text.push_line(Line::from(vec![Span::styled(
                "Misconceptions:",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )]));
            for misconception in &assessment.misconceptions {
                let rendered = render_markdown(&format!("  • {}", misconception));
                content_text.extend(rendered);
            }
            content_text.push_line(Line::from(""));
        }

        if !assessment.key_concepts_to_review.is_empty() {
            content_text.push_line(Line::from(vec![Span::styled(
                "Key Concepts to Review:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]));
            for concept in &assessment.key_concepts_to_review {
                let rendered = render_markdown(&format!("  • {}", concept));
                content_text.extend(rendered);
            }
            content_text.push_line(Line::from(""));
        }

        if !assessment.priority_questions.is_empty() {
            content_text.push_line(Line::from(vec![Span::styled(
                "Questions to Revisit:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]));
            for question in &assessment.priority_questions {
                let rendered = render_markdown(&format!("  • {}", question));
                content_text.extend(rendered);
            }
            content_text.push_line(Line::from(""));
        }

        if !assessment.study_focus.is_empty() {
            content_text.push_line(Line::from(vec![
                Span::styled(
                    "Next Session Focus: ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&assessment.study_focus),
            ]));
        }
    } else if let Some(ref error) = session.assessment_error {
        let score_color = if avg_score >= 80.0 {
            Color::Green
        } else if avg_score >= 50.0 {
            Color::Yellow
        } else {
            Color::Red
        };

        content_text.push_line(Line::from(vec![
            Span::raw("Answered: "),
            Span::styled(
                format!("{}/{}", answered_count, session.questions_total),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  Score: "),
            Span::styled(
                format!("{:.0}%", avg_score),
                Style::default()
                    .fg(score_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        content_text.push_line(Line::from(""));
        content_text.push_line(Line::from(Span::styled(
            "Analysis unavailable",
            Style::default().fg(Color::Red),
        )));
        content_text.push_line(Line::from(error.as_str()));
    } else {
        let score_color = if avg_score >= 80.0 {
            Color::Green
        } else if avg_score >= 50.0 {
            Color::Yellow
        } else {
            Color::Red
        };

        content_text.push_line(Line::from(vec![
            Span::raw("Answered: "),
            Span::styled(
                format!("{}/{}", answered_count, session.questions_total),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  Score: "),
            Span::styled(
                format!("{:.0}%", avg_score),
                Style::default()
                    .fg(score_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        content_text.push_line(Line::from(""));
        content_text.push_line(Line::from(Span::styled(
            "No AI analysis available",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let visible_height = layout.content_area.height.saturating_sub(2) as usize;
    let text_width = layout.content_area.width.saturating_sub(2) as usize;
    let content_height = estimate_text_height(&content_text, text_width);
    let max_scroll = calculate_max_scroll(content_height, visible_height);
    session.assessment_scroll_y = session.assessment_scroll_y.min(max_scroll);

    let content_widget = Paragraph::new(content_text)
        .wrap(Wrap { trim: true })
        .scroll((session.assessment_scroll_y, 0))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(content_widget, layout.content_area);

    // Footer with all keybindings
    let mut help_spans = vec![
        Span::styled(
            "m",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Menu  "),
        Span::styled(
            "Esc",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Quit"),
    ];

    // Add retry option if AI is enabled
    if session.ai_enabled {
        help_spans.push(Span::from("  "));
        help_spans.push(Span::styled(
            "r",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        help_spans.push(Span::from(" Retry Analysis"));
    }

    let help = Paragraph::new(vec![Line::from(help_spans)])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, layout.footer_area);
}
