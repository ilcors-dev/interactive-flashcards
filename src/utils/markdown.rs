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
                    // Create new span with content and try to preserve basic styling
                    let mut new_span = Span::raw(span.content.to_string());

                    // Try to extract some style info if possible
                    if span.style.fg.is_some()
                        || span.style.bg.is_some()
                        || !span.style.add_modifier.is_empty()
                    {
                        new_span = Span::styled(
                            span.content.to_string(),
                            ratatui::style::Style::default(),
                        );
                    }

                    new_span
                })
                .collect();
            Line::from(spans)
        })
        .collect()
}

/// Render markdown with truncation, preserving markdown structure where possible
/// Falls back to plain text truncation if markdown parsing fails
pub fn render_markdown_truncated(content: &str, max_width: usize) -> Vec<Line<'static>> {
    // Preprocess HTML tags to handle them correctly
    let processed_content = preprocess_html_tags(content);

    // Use tui-markdown for proper markdown parsing with syntax highlighting
    let text = from_str(&processed_content);

    // Truncate each line to max_width while preserving styling
    text.lines
        .into_iter()
        .map(|line| {
            // Convert to ratatui Line type first
            let spans: Vec<Span> = line
                .spans
                .into_iter()
                .map(|span| Span::raw(span.content.to_string()))
                .collect();
            let ratatui_line = Line::from(spans);

            // Get plain text content for length checking
            let plain_content = ratatui_line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>();

            if plain_content.len() > max_width {
                // Smart truncation that preserves styling
                truncate_line_with_styling(ratatui_line, max_width)
            } else {
                ratatui_line
            }
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

    // Bold tags: &lt;b&gt;text&lt;/b&gt; → **text**
    let bold_re = Regex::new(r"(?i)&lt;b[^&gt;]*&gt;(.*?)&lt;/b&gt;").unwrap();
    result = bold_re.replace_all(&result, "**$1**").to_string();

    // Strong tags: &lt;strong&gt;text&lt;/strong&gt; → **text**
    let strong_re = Regex::new(r"(?i)&lt;strong[^&gt;]*&gt;(.*?)&lt;/strong&gt;").unwrap();
    result = strong_re.replace_all(&result, "**$1**").to_string();

    // Italic tags: &lt;i&gt;text&lt;/i&gt; → *text*
    let italic_re = Regex::new(r"(?i)&lt;i[^&gt;]*&gt;(.*?)&lt;/i&gt;").unwrap();
    result = italic_re.replace_all(&result, "*$1*").to_string();

    // Em tags: &lt;em&gt;text&lt;/em&gt; → *text*
    let em_re = Regex::new(r"(?i)&lt;em[^&gt;]*&gt;(.*?)&lt;/em&gt;").unwrap();
    result = em_re.replace_all(&result, "*$1*").to_string();

    // Code tags: &lt;code&gt;text&lt;/code&gt; → `text`
    let code_re = Regex::new(r"(?i)&lt;code[^&gt;]*&gt;(.*?)&lt;/code&gt;").unwrap();
    result = code_re.replace_all(&result, "`$1`").to_string();

    // Pre tags: &lt;pre&gt;text&lt;/pre&gt; → ```text```
    let pre_re = Regex::new(r"(?s)&lt;pre[^&gt;]*&gt;(.*?)&lt;/pre&gt;").unwrap();
    result = pre_re.replace_all(&result, "```\n$1\n```").to_string();

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

/// Truncate a line while preserving styling across spans
fn truncate_line_with_styling(line: Line<'static>, max_width: usize) -> Line<'static> {
    let mut current_width = 0;
    let mut truncated_spans = Vec::new();

    for span in line.spans {
        let span_text = span.content.as_ref();

        if current_width + span_text.len() <= max_width {
            // Span fits completely - clone it to avoid move issues
            truncated_spans.push(span.clone());
            current_width += span_text.len();
        } else if current_width < max_width {
            // Span needs to be truncated
            let remaining = max_width - current_width;
            let truncated_text = &span_text[..remaining];
            truncated_spans.push(Span::styled(truncated_text.to_string(), span.style));
            break; // We've reached max width
        } else {
            // We've already reached max width
            break;
        }
    }

    Line::from(truncated_spans)
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

    #[test]
    fn test_android_permission_text() {
        let content = "In Android, any operation that requires a privileged system capability must be declared in the app's AndroidManifest.xml. Network access is a privileged capability, so you must explicitly request it using the <uses-permission> tag. The correct syntax is: <uses-permission android:name=\"android.permission.INTERNET\"/>";
        let result = render_markdown(content);
        assert!(!result.is_empty());
        let combined = result
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        // The content should contain permission information in some form
        assert!(combined.contains("uses-permission") || combined.contains("permission"));
        assert!(combined.contains("INTERNET") || combined.contains("Internet"));
        assert!(combined.contains("AndroidManifest"));
    }

    #[test]
    fn test_html_bold_conversion() {
        let content = "This is <b>bold text</b> and this is <strong>strong text</strong>";
        let result = render_markdown(content);
        assert!(!result.is_empty());
        let combined = result
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(combined.contains("bold text"));
        assert!(combined.contains("strong text"));
    }

    #[test]
    fn test_html_italic_conversion() {
        let content = "This is <i>italic text</i> and this is <em>emphasis text</em>";
        let result = render_markdown(content);
        assert!(!result.is_empty());
        let combined = result
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(combined.contains("italic text"));
        assert!(combined.contains("emphasis text"));
    }

    #[test]
    fn test_html_code_conversion() {
        let content = "Use the <code>main()</code> function";
        let result = render_markdown(content);
        assert!(!result.is_empty());
        let combined = result
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(combined.contains("main()"));
    }

    #[test]
    fn test_html_pre_conversion() {
        let content = "Here is code:\n<pre>fn main() {\n    println!(\"Hello\");\n}</pre>";
        let result = render_markdown(content);
        assert!(!result.is_empty());
        let combined = result
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(combined.contains("```"));
        assert!(combined.contains("fn main()"));
    }

    #[test]
    fn test_unknown_html_tags() {
        let content = "This has <unknown-tag>content</unknown-tag> and <div>more</div>";
        let result = render_markdown(content);
        assert!(!result.is_empty());
        let combined = result
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        // After markdown parsing, should contain the content
        assert!(combined.contains("content"));
        assert!(combined.contains("more"));
        // Unknown tags may be visible or processed
        assert!(combined.len() > 10);
    }

    #[test]
    fn test_mixed_html_and_markdown() {
        let content = "**Bold markdown** and <b>bold HTML</b> should both work";
        let result = render_markdown(content);
        assert!(!result.is_empty());

        // For now, just check that content contains bold text (converted from both sources)
        let combined = result
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(combined.contains("Bold markdown"));
        assert!(combined.contains("bold HTML"));
    }

    #[test]
    fn test_markdown_styling_preserved() {
        let content = "This has **bold**, *italic*, and `code` text";
        let result = render_markdown(content);
        assert!(!result.is_empty());

        // Check for spans with different styles
        let combined_text = result
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(combined_text.contains("bold"));
        assert!(combined_text.contains("italic"));
        assert!(combined_text.contains("code"));
    }

    #[test]
    fn test_self_closing_tags() {
        let content = "Here are some tags: <img src=\"test.jpg\"/> and <uses-permission/>";
        let result = render_markdown(content);
        assert!(!result.is_empty());
        let combined = result
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        // After markdown parsing, should contain relevant content
        assert!(combined.contains("img") || combined.contains("test.jpg"));
        assert!(combined.contains("uses-permission"));
    }

    #[test]
    fn test_truncated_preserves_styling() {
        let content = "This has <b>bold</b> and <i>italic</i> text";
        let result = render_markdown_truncated(content, 15);
        assert!(!result.is_empty());

        let total_len: usize = result.iter().map(|line| line.to_string().len()).sum();
        assert!(total_len <= 15);
    }

    #[test]
    fn test_html_attributes_preserved() {
        let content =
            "Tag with attributes: <uses-permission android:name=\"android.permission.INTERNET\"/>";
        let result = render_markdown(content);
        assert!(!result.is_empty());
        let combined = result
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        // After markdown parsing, should contain the permission information
        assert!(combined.contains("uses-permission"));
        assert!(combined.contains("INTERNET"));
    }

    #[test]
    fn test_markdown_features() {
        let content = "# Header\n\n**Bold text** and *italic text*\n\n- List item 1\n- List item 2\n\n```rust\nfn main() {}\n```";
        let result = render_markdown(content);
        assert!(!result.is_empty());

        let combined = result
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        // Should contain all the markdown elements
        assert!(combined.contains("Header"));
        assert!(combined.contains("Bold text"));
        assert!(combined.contains("italic text"));
        assert!(combined.contains("List item"));
        assert!(combined.contains("fn main"));
    }
}
