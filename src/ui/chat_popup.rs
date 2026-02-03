use crate::models::{ChatRole, ChatState};
use crate::utils::{calculate_max_scroll, estimate_text_height, render_markdown};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Rebuild the rendered lines cache from messages.
/// This is the expensive operation (markdown parsing) that we want to avoid on every frame.
pub fn rebuild_chat_cache(chat: &mut ChatState) {
    let mut lines: Vec<Line<'static>> = Vec::new();

    for msg in &chat.messages {
        match msg.role {
            ChatRole::User => {
                lines.push(Line::from(Span::styled(
                    "You:",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )));
                for line in msg.content.lines() {
                    lines.push(Line::from(format!("  {}", line)));
                }
                lines.push(Line::from(""));
            }
            ChatRole::Assistant => {
                lines.push(Line::from(Span::styled(
                    "AI:",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )));
                let rendered = render_markdown(&msg.content);
                for line in rendered {
                    let mut indented_spans: Vec<Span<'static>> = vec![Span::from("  ")];
                    indented_spans.extend(line.spans);
                    lines.push(Line::from(indented_spans));
                }
                lines.push(Line::from(""));
            }
            ChatRole::System => {
                lines.push(Line::from(Span::styled(
                    msg.content.clone(),
                    Style::default().fg(Color::DarkGray),
                )));
                lines.push(Line::from(""));
            }
        }
    }

    chat.rendered_lines_cache = lines;
    chat.cached_message_count = chat.messages.len();
}

pub fn draw_chat_popup(f: &mut Frame, chat: &mut ChatState, question_number: usize) {
    let area = centered_rect(80, 85, f.area());

    f.render_widget(Clear, area);

    let title = if chat.read_only {
        format!(" Chat - Q{} (Read Only) ", question_number)
    } else {
        format!(" Chat - Q{} ", question_number)
    };

    // Split popup into messages area, input area, and help line
    let input_height = if chat.read_only { 0 } else { 3 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(input_height),
            Constraint::Length(1),
        ])
        .split(area);

    // Rebuild cache only if messages changed
    if chat.cached_message_count != chat.messages.len() {
        rebuild_chat_cache(chat);
    }

    // Start with cached lines (clone is cheap - just reference counting for the inner strings)
    let mut message_lines: Vec<Line<'static>> = chat.rendered_lines_cache.clone();

    // Add dynamic elements (loading indicator, errors) - these are cheap
    if chat.is_loading {
        message_lines.push(Line::from(Span::styled(
            "AI is thinking...",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::ITALIC),
        )));
    }

    if let Some(err) = &chat.error {
        message_lines.push(Line::from(Span::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red),
        )));
    }

    if message_lines.is_empty() {
        message_lines.push(Line::from(Span::styled(
            "Start a conversation about this question...",
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Calculate scroll bounds accounting for line wrapping
    let visible_height = chunks[0].height.saturating_sub(2) as usize;
    let text_width = chunks[0].width.saturating_sub(2) as usize;
    let content_text = Text::from(message_lines);
    let content_height = estimate_text_height(&content_text, text_width);
    // Add generous buffer (50%) to account for word wrapping overhead not captured by estimation
    let buffered_height = content_height + content_height / 2;
    let max_scroll = calculate_max_scroll(buffered_height, visible_height);

    // Store max_scroll for bounds checking in event handlers
    chat.max_scroll = max_scroll;

    // Auto-scroll to bottom when loading, otherwise use user's scroll position
    let scroll = if chat.is_loading {
        max_scroll
    } else {
        chat.scroll_y.min(max_scroll)
    };
    chat.scroll_y = scroll;

    let messages_widget = Paragraph::new(content_text)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Cyan)),
        );
    f.render_widget(messages_widget, chunks[0]);

    // Input area (hidden in read-only mode)
    if !chat.read_only {
        let input_text = if chat.input_buffer.is_empty() && !chat.is_loading {
            Text::from(Span::styled(
                "Type your message...",
                Style::default().fg(Color::DarkGray),
            ))
        } else {
            Text::from(chat.input_buffer.as_str())
        };

        let input_widget = Paragraph::new(input_text).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Message ")
                .border_style(if chat.is_loading {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::Yellow)
                }),
        );
        f.render_widget(input_widget, chunks[1]);

        // Set cursor in input area
        if !chat.is_loading {
            let text_width = (chunks[1].width.saturating_sub(2)) as usize;
            let (cursor_line, cursor_col) = crate::calculate_wrapped_cursor_position(
                &chat.input_buffer,
                chat.cursor_position,
                text_width,
            );
            let cursor_x = chunks[1].x + 1 + cursor_col as u16;
            let cursor_y = chunks[1].y + 1 + cursor_line as u16;
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }

    // Help line
    let help_spans = if chat.read_only {
        vec![
            Span::styled(
                "Ctrl+T",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::from("/"),
            Span::styled(
                "Esc",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::from(" Close  "),
            Span::styled(
                "↑/↓",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::from(" Scroll"),
        ]
    } else {
        vec![
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::from(" Send  "),
            Span::styled(
                "Ctrl+T",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::from("/"),
            Span::styled(
                "Esc",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::from(" Close  "),
            Span::styled(
                "↑/↓",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::from(" Scroll"),
        ]
    };

    let help = Paragraph::new(Line::from(help_spans))
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[2]);
}
