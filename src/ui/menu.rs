use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::path::PathBuf;

use crate::ai::DEFAULT_MODEL;
use crate::db::session::SessionSummary;

fn format_session_date(timestamp: u64) -> String {
    use std::time::{Duration, UNIX_EPOCH};

    let session_time = UNIX_EPOCH + Duration::from_secs(timestamp);
    let datetime: chrono::DateTime<chrono::Local> = session_time.into();

    let today = chrono::Local::now().date_naive();
    let session_date = datetime.date_naive();

    if session_date == today {
        let time_str = datetime.format("%H:%M").to_string();
        format!("Today {}", time_str)
    } else if session_date == today - chrono::Duration::days(1) {
        let time_str = datetime.format("%H:%M").to_string();
        format!("Yesterday {}", time_str)
    } else {
        session_date.format("%Y-%m-%d").to_string()
    }
}

fn format_session_item(session: &SessionSummary) -> String {
    let date = format_session_date(session.started_at);
    let status = if session.completed_at.is_some() {
        "COMPLETED".to_string()
    } else {
        format!("{}/{}", session.questions_answered, session.questions_total)
    };
    format!("{} - {} ({})", date, session.deck_name, status)
}

fn draw_panel_header(area: ratatui::layout::Rect, title: &str, focused: bool, f: &mut Frame) {
    let style = if focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let header = Paragraph::new(title)
        .style(style)
        .alignment(Alignment::Left)
        .block(Block::default());

    f.render_widget(header, area);
}

pub fn draw_menu(
    f: &mut Frame,
    csv_files: &[PathBuf],
    selected_file_index: usize,
    sessions: &[SessionSummary],
    selected_session_index: usize,
    focused_panel: usize,
    ai_enabled: bool,
) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new("Interactive Flashcards v0.1.0")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let csv_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(chunks[1]);

    let sessions_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(chunks[2]);

    draw_panel_header(csv_chunks[0], "[1] CSV Files", focused_panel == 0, f);

    let csv_items: Vec<ListItem> = if csv_files.is_empty() {
        vec![ListItem::new("No CSV files found").style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )]
    } else {
        csv_files
            .iter()
            .enumerate()
            .map(|(i, path)| {
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                let style = if i == selected_file_index && focused_panel == 0 {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(name).style(style)
            })
            .collect()
    };

    let csv_list = List::new(csv_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if focused_panel == 0 {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_widget(csv_list, csv_chunks[1]);

    draw_panel_header(sessions_chunks[0], "[2] Sessions", focused_panel == 1, f);

    let session_items: Vec<ListItem> = if sessions.is_empty() {
        vec![ListItem::new("No past sessions").style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )]
    } else {
        sessions
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let text = format_session_item(session);
                let style = if i == selected_session_index && focused_panel == 1 {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(text).style(style)
            })
            .collect()
    };

    let sessions_list = List::new(session_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if focused_panel == 1 {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_widget(sessions_list, sessions_chunks[1]);

    let help_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[3]);

    let ai_status_content = if ai_enabled {
        vec![
            Line::from("AI: Enabled"),
            Line::from(format!("Model: {}", DEFAULT_MODEL)),
        ]
    } else {
        vec![
            Line::from("AI: Disabled"),
            Line::from("Set OPENROUTER_API_KEY"),
        ]
    };

    let ai_status = Paragraph::new(ai_status_content)
        .style(
            Style::default()
                .fg(if ai_enabled {
                    Color::Green
                } else {
                    Color::Yellow
                })
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::ALL).title("AI Status"));
    f.render_widget(ai_status, help_chunks[0]);

    let help_text = vec![Line::from(vec![
        Span::styled(
            "1/2",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Focus Panel  "),
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
            "Esc/Ctrl+C",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(" Quit"),
    ])];
    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, help_chunks[1]);
}
