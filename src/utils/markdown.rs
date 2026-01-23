use ratatui::text::Line;
use tui_markdown::from_str;

/// Render markdown content to Vec<Line> for ratatui
/// Falls back to plain text rendering if markdown parsing fails
pub fn render_markdown(content: &str) -> Vec<Line<'static>> {
    // Try to parse as markdown, fall back to plain text on error
    match std::panic::catch_unwind(|| from_str(content)) {
        Ok(text) => {
            // Convert tui_markdown Lines to ratatui Lines (simplified - just plain text for now)
            text.lines
                .into_iter()
                .map(|md_line| {
                    // For now, just concatenate all spans into plain text
                    // TODO: Preserve styling when ratatui versions are compatible
                    let plain_text = md_line
                        .spans
                        .iter()
                        .map(|span| span.content.as_ref())
                        .collect::<String>();
                    Line::from(plain_text)
                })
                .collect()
        }
        Err(_) => {
            // Fallback: render as plain text with line breaks
            content
                .lines()
                .map(|line| Line::from(line.to_string()))
                .collect()
        }
    }
}

/// Render markdown with truncation, preserving markdown structure where possible
/// Falls back to plain text truncation if markdown parsing fails
pub fn render_markdown_truncated(content: &str, max_width: usize) -> Vec<Line<'static>> {
    let rendered = render_markdown(content);

    // Truncate each line to max_width
    rendered
        .into_iter()
        .map(|line| {
            // Get the plain text content for length checking
            let plain_content = line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>();

            if plain_content.len() > max_width {
                // Simple truncation to max_width characters
                let truncated_content = plain_content[..max_width].to_string();
                Line::from(truncated_content)
            } else {
                line
            }
        })
        .collect()
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
        // Markdown parser may combine lines, so we just check that content is present
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
    fn test_render_markdown_truncated() {
        let content = "This is a very long explanation that should be truncated";
        let result = render_markdown_truncated(content, 20);
        let combined = result
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("");
        assert!(combined.len() <= 20);
    }
}
