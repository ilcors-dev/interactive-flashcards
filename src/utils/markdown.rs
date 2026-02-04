use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use regex::Regex;

/// Render markdown content to Vec<Line> for ratatui.
/// Supports: **bold**, *italic*, `code`, - / * / numbered lists, ### headings,
/// and | pipe | tables |.
pub fn render_markdown(content: &str) -> Vec<Line<'static>> {
    let lines: Vec<&str> = content.lines().collect();
    let mut result: Vec<Line<'static>> = Vec::new();
    let mut i = 0;
    let numbered_re = Regex::new(r"^(\d+)\.\s+(.*)$").unwrap();

    while i < lines.len() {
        let line = lines[i];

        // Detect markdown table
        if is_table_row(line) && i + 1 < lines.len() && is_table_separator(lines[i + 1]) {
            let mut table_rows: Vec<Vec<String>> = Vec::new();
            table_rows.push(parse_table_row(line));
            i += 2; // skip header + separator

            while i < lines.len() && is_table_row(lines[i]) && !is_table_separator(lines[i]) {
                table_rows.push(parse_table_row(lines[i]));
                i += 1;
            }

            render_table(&table_rows, &mut result);
            continue;
        }

        let trimmed = line.trim();

        // Headings
        if let Some(heading) = trimmed.strip_prefix("### ") {
            result.push(Line::from(Span::styled(
                heading.to_string(),
                Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            i += 1;
            continue;
        }
        if let Some(heading) = trimmed.strip_prefix("## ") {
            result.push(Line::from(Span::styled(
                heading.to_string(),
                Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            i += 1;
            continue;
        }
        if let Some(heading) = trimmed.strip_prefix("# ") {
            result.push(Line::from(Span::styled(
                heading.to_string(),
                Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            i += 1;
            continue;
        }

        // Unordered list items (- or *)
        if let Some(item) = trimmed.strip_prefix("- ").or(trimmed.strip_prefix("* ")) {
            let mut spans = vec![Span::from("  • ")];
            spans.extend(parse_inline(item));
            result.push(Line::from(spans));
            i += 1;
            continue;
        }

        // Numbered list items
        if let Some(caps) = numbered_re.captures(trimmed) {
            let num = caps.get(1).unwrap().as_str();
            let item = caps.get(2).unwrap().as_str();
            let mut spans = vec![Span::from(format!("  {}. ", num))];
            spans.extend(parse_inline(item));
            result.push(Line::from(spans));
            i += 1;
            continue;
        }

        // Regular line with inline formatting
        if trimmed.is_empty() {
            result.push(Line::from(""));
        } else {
            result.push(Line::from(parse_inline(line)));
        }
        i += 1;
    }

    result
}

/// Parse inline markdown: **bold**, *italic*, `code`
fn parse_inline(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut remaining = text;

    // Regex for inline patterns: **bold**, *italic*, `code`
    let inline_re = Regex::new(r"(\*\*(.+?)\*\*|\*(.+?)\*|`([^`]+)`)").unwrap();

    while !remaining.is_empty() {
        if let Some(m) = inline_re.find(remaining) {
            // Add text before match
            if m.start() > 0 {
                spans.push(Span::from(remaining[..m.start()].to_string()));
            }

            let matched = m.as_str();
            let caps = inline_re.captures(matched).unwrap();

            if let Some(bold) = caps.get(2) {
                // **bold**
                spans.push(Span::styled(
                    bold.as_str().to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
            } else if let Some(italic) = caps.get(3) {
                // *italic*
                spans.push(Span::styled(
                    italic.as_str().to_string(),
                    Style::default().add_modifier(Modifier::ITALIC),
                ));
            } else if let Some(code) = caps.get(4) {
                // `code`
                spans.push(Span::styled(
                    code.as_str().to_string(),
                    Style::default().add_modifier(Modifier::DIM),
                ));
            }

            remaining = &remaining[m.end()..];
        } else {
            spans.push(Span::from(remaining.to_string()));
            break;
        }
    }

    if spans.is_empty() {
        spans.push(Span::from(text.to_string()));
    }

    spans
}

fn is_table_row(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.contains('|') && !trimmed.is_empty()
}

fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.contains('|')
        && trimmed
            .chars()
            .all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
}

fn parse_table_row(line: &str) -> Vec<String> {
    let trimmed = line.trim().trim_matches('|');
    trimmed
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

/// Render a table as readable lines without fixed-width columns.
/// Each data row becomes a block of "Header: Value" lines so it wraps naturally.
fn render_table(rows: &[Vec<String>], output: &mut Vec<Line<'static>>) {
    if rows.is_empty() {
        return;
    }

    let headers: &Vec<String> = &rows[0];
    let data_rows = &rows[1..];

    if data_rows.is_empty() {
        // Header only — render as bold line
        let header_text = headers.join(" │ ");
        output.push(Line::from(Span::styled(
            header_text,
            Style::default().add_modifier(Modifier::BOLD),
        )));
        return;
    }

    for (row_idx, row) in data_rows.iter().enumerate() {
        if row_idx > 0 {
            output.push(Line::from(""));
        }
        for (j, cell) in row.iter().enumerate() {
            let header = headers.get(j).map(|s| s.as_str()).unwrap_or("?");
            let mut spans = vec![Span::styled(
                format!("{}: ", header),
                Style::default().add_modifier(Modifier::BOLD),
            )];
            spans.extend(parse_inline(cell));
            output.push(Line::from(spans));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_markdown_plain_text() {
        let content = "Hello world";
        let result = render_markdown(content);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].to_string(), "Hello world");
    }

    #[test]
    fn test_render_markdown_multiline() {
        let content = "Line 1\nLine 2";
        let result = render_markdown(content);
        assert!(!result.is_empty());
        let combined = result
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(combined.contains("Line 1"));
        assert!(combined.contains("Line 2"));
    }

    #[test]
    fn test_table_detection() {
        assert!(is_table_row("| A | B |"));
        assert!(is_table_row("| A | B"));
        assert!(!is_table_row("no pipes here"));
        assert!(is_table_separator("|---|---|"));
        assert!(is_table_separator("| --- | :---: |"));
        assert!(!is_table_separator("| A | B |"));
    }

    #[test]
    fn test_parse_table_row() {
        let row = parse_table_row("| Hello | World |");
        assert_eq!(row, vec!["Hello", "World"]);
    }

    #[test]
    fn test_render_markdown_with_table() {
        let input = "| Col1 | Col2 |\n|------|------|\n| A | B |";
        let result = render_markdown(input);
        let combined: String = result
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(combined.contains("Col1"));
        assert!(combined.contains("A"));
        assert!(combined.contains("B"));
    }

    #[test]
    fn test_table_mixed_with_text() {
        let input = "Before\n\n| X | Y |\n|---|---|\n| 1 | 2 |\n\nAfter";
        let result = render_markdown(input);
        let combined: String = result
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(combined.contains("Before"));
        assert!(combined.contains("After"));
        assert!(combined.contains("1"));
        assert!(combined.contains("2"));
    }

    #[test]
    fn test_markdown_styling_actually_preserved() {
        let content = "**bold**";
        let result = render_markdown(content);
        let line = &result[0];
        assert_eq!(line.spans.len(), 1);
        let span = &line.spans[0];
        assert!(span
            .style
            .add_modifier
            .intersects(ratatui::style::Modifier::BOLD));
    }

    #[test]
    fn test_italic_rendering() {
        let content = "*italic*";
        let result = render_markdown(content);
        let line = &result[0];
        assert_eq!(line.spans.len(), 1);
        assert!(line.spans[0]
            .style
            .add_modifier
            .intersects(Modifier::ITALIC));
    }

    #[test]
    fn test_code_rendering() {
        let content = "`code`";
        let result = render_markdown(content);
        let line = &result[0];
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content, "code");
    }

    #[test]
    fn test_mixed_inline() {
        let content = "Hello **bold** and *italic* world";
        let result = render_markdown(content);
        let line = &result[0];
        // Should have: "Hello " + bold + " and " + italic + " world"
        assert!(line.spans.len() >= 5);
        assert_eq!(line.spans[0].content, "Hello ");
        assert!(line.spans[1].style.add_modifier.intersects(Modifier::BOLD));
        assert_eq!(line.spans[1].content, "bold");
    }

    #[test]
    fn test_unordered_list() {
        let content = "- Item 1\n- Item 2\n* Item 3";
        let result = render_markdown(content);
        assert_eq!(result.len(), 3);
        assert!(result[0].to_string().contains("•"));
        assert!(result[0].to_string().contains("Item 1"));
        assert!(result[2].to_string().contains("Item 3"));
    }

    #[test]
    fn test_numbered_list() {
        let content = "1. First\n2. Second";
        let result = render_markdown(content);
        assert_eq!(result.len(), 2);
        assert!(result[0].to_string().contains("1."));
        assert!(result[0].to_string().contains("First"));
    }

    #[test]
    fn test_heading() {
        let content = "### My Heading";
        let result = render_markdown(content);
        assert_eq!(result.len(), 1);
        assert!(result[0].spans[0]
            .style
            .add_modifier
            .intersects(Modifier::BOLD));
        assert_eq!(result[0].spans[0].content, "My Heading");
    }

    #[test]
    fn test_bold_in_list() {
        let content = "- **Important** item";
        let result = render_markdown(content);
        let line = &result[0];
        let text = line.to_string();
        assert!(text.contains("•"));
        assert!(text.contains("Important"));
    }

    #[test]
    fn test_empty_lines() {
        let content = "Line 1\n\nLine 2";
        let result = render_markdown(content);
        assert_eq!(result.len(), 3);
        assert_eq!(result[1].to_string(), "");
    }
}
