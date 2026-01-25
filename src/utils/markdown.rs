use ratatui::text::{Line, Span};
use regex::Regex;
use tui_markdown::from_str;

/// Render markdown content to Vec<Line> for ratatui
/// Falls back to plain text rendering if markdown parsing fails
pub fn render_markdown(content: &str) -> Vec<Line<'static>> {
    // Preprocess HTML tags to handle them correctly
    let processed_content = preprocess_html_tags(content);

    // Use tui-markdown for proper markdown parsing with syntax highlighting
    let text = from_str(&processed_content);

    // Convert by recreating spans from the string content and styling info
    text.lines
        .into_iter()
        .map(|line| {
            let spans: Vec<Span> = line
                .spans
                .into_iter()
                .map(|span| {
                    // Create new span with content and preserve modifiers (colors are complex to convert)
                    let add_mods = ratatui::style::Modifier::from_bits_truncate(span.style.add_modifier.bits());
                    let new_style = ratatui::style::Style::default().add_modifier(add_mods);
                    Span::styled(span.content.to_string(), new_style)
                })
                .collect();
            Line::from(spans)
        })
        .collect()
}

/// Preprocess HTML tags to handle them correctly in markdown rendering
fn preprocess_html_tags(content: &str) -> String {
    let mut processed = content.to_string();

    // First, escape all unknown HTML tags to display as plain text
    // This handles tags like <uses-permission> and <unknown-tag>
    processed = escape_all_html_tags(&processed);

    // Then convert supported HTML tags to markdown equivalents
    processed = convert_supported_html_to_markdown(&processed);

    processed
}

/// Convert supported HTML tags to their markdown equivalents
fn convert_supported_html_to_markdown(content: &str) -> String {
    let mut result = content.to_string();

    // Bold tags: <b>text</b> → **text**
    let bold_re = Regex::new(r"(?i)<b[^>]*>(.*?)</b>").unwrap();
    result = bold_re.replace_all(&result, "**$1**").to_string();

    // Strong tags: <strong>text</strong> → **text**
    let strong_re = Regex::new(r"(?i)<strong[^>]*>(.*?)</strong>").unwrap();
    result = strong_re.replace_all(&result, "**$1**").to_string();

    // Italic tags: <i>text</i> → *text*
    let italic_re = Regex::new(r"(?i)<i[^>]*>(.*?)</i>").unwrap();
    result = italic_re.replace_all(&result, "*$1*").to_string();

    // Em tags: <em>text</em> → *text*
    let em_re = Regex::new(r"(?i)<em[^>]*>(.*?)</em>").unwrap();
    result = em_re.replace_all(&result, "*$1*").to_string();

    // Code tags: <code>text</code> → `text`
    let code_re = Regex::new(r"(?i)<code[^>]*>(.*?)</code>").unwrap();
    result = code_re.replace_all(&result, "`$1`").to_string();

    // Pre tags: <pre>text</pre> → ```text```
    let pre_re = Regex::new(r"(?s)<pre[^>]*>(.*?)</pre>").unwrap();
    result = pre_re.replace_all(&result, "```\n$1\n```").to_string();

    // Blockquote tags: <blockquote>text</blockquote> → > text
    let blockquote_re = Regex::new(r"(?s)<blockquote[^>]*>(.*?)</blockquote>").unwrap();
    result = blockquote_re.replace_all(&result, "> $1").to_string();

    result
}

/// Escape all HTML tags to display as plain text
fn escape_all_html_tags(content: &str) -> String {
    let mut result = content.to_string();

    // First escape quotes globally
    result = result.replace('"', "&quot;");

    // Then escape HTML tag brackets
    let html_tag_re = Regex::new(r"<(/?)([a-zA-Z][a-zA-Z0-9:-]*)([^>]*?)>").unwrap();
    html_tag_re
        .replace_all(&result, "&lt;$1$2$3&gt;")
        .to_string()
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
    fn test_markdown_styling_actually_preserved() {
        // This test checks that styles are preserved, not just content
        let content = "**bold**";
        let result = render_markdown(content);
        let line = &result[0];
        assert_eq!(line.spans.len(), 1);
        let span = &line.spans[0];
        assert!(span.style.add_modifier.intersects(ratatui::style::Modifier::BOLD));
    }
}
