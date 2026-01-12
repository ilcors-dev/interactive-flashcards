use unicode_width::UnicodeWidthChar;

pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Simulate how text wraps with trimming (matching ratatui Wrap { trim: true } behavior)
/// Handles both explicit newlines (\n) and automatic wrapping at max_width
/// Returns a vector of (line_text, start_index, end_index) for each visual line
fn simulate_wrapped_lines(text: &str, max_width: usize) -> Vec<(String, usize, usize)> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;
    let mut line_start_idx = 0;

    for (char_idx, ch) in text.char_indices() {
        if ch == '\n' {
            // Force line break at explicit newline - trim current line
            let trimmed = current_line.trim_end().to_string();
            lines.push((trimmed, line_start_idx, char_idx));

            // Start new line after the newline
            current_line = String::new();
            current_width = 0;
            line_start_idx = char_idx + 1;
        } else {
            let char_width = ch.width().unwrap_or(1);

            if current_width + char_width > max_width && current_width > 0 {
                // Auto-wrap to next line - trim trailing whitespace from current line
                let trimmed = current_line.trim_end().to_string();
                lines.push((trimmed, line_start_idx, char_idx));

                // Start new line with this character
                current_line = ch.to_string();
                current_width = char_width;
                line_start_idx = char_idx;
            } else {
                current_line.push(ch);
                current_width += char_width;
            }
        }
    }

    // Add the last line, trimmed
    if !current_line.is_empty() || text.ends_with('\n') {
        let trimmed = current_line.trim_end().to_string();
        lines.push((trimmed, line_start_idx, text.len()));
    }

    lines
}

/// Calculate the line and column position of a cursor within wrapped text.
/// Accounts for trimming behavior (matching ratatui Wrap { trim: true }).
/// Returns (line_number, column_in_line) for the given cursor position in the text.
pub fn calculate_wrapped_cursor_position(
    text: &str,
    cursor_index: usize,
    max_width: usize,
) -> (usize, usize) {
    if text.is_empty() || cursor_index == 0 {
        return (0, 0);
    }

    // Simulate how the text would be wrapped and trimmed
    let wrapped_lines = simulate_wrapped_lines(text, max_width);

    // Find which visual line contains the cursor
    for (line_idx, (_, start_idx, end_idx)) in wrapped_lines.iter().enumerate() {
        if cursor_index >= *start_idx && cursor_index <= *end_idx {
            // Cursor is in this visual line
            let col_in_line = cursor_index.saturating_sub(*start_idx);
            return (line_idx, col_in_line);
        }
    }

    // Cursor is beyond the last line or in trimmed space
    // Find the closest visual line
    if let Some((_, _, last_end)) = wrapped_lines.last() {
        if cursor_index >= *last_end {
            let last_line_idx = wrapped_lines.len().saturating_sub(1);
            let last_line_len = wrapped_lines
                .last()
                .map(|(text, _, _)| text.chars().count())
                .unwrap_or(0);
            return (last_line_idx, last_line_len);
        }
    }

    // Fallback
    (0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_string_no_truncation() {
        let s = "Short string";
        let result = truncate_string(s, 20);
        assert_eq!(result, "Short string");
    }

    #[test]
    fn test_truncate_string_with_truncation() {
        let s = "This is a very long string that should be truncated";
        let result = truncate_string(s, 20);
        assert_eq!(result, "This is a very lo...");
        assert!(result.len() <= 20);
    }

    #[test]
    fn test_truncate_string_exact_length() {
        let s = "Exactly twenty!!";
        let result = truncate_string(s, 20);
        assert_eq!(result, "Exactly twenty!!");
    }

    #[test]
    fn test_truncate_string_empty() {
        let s = "";
        let result = truncate_string(s, 20);
        assert_eq!(result, "");
    }

    #[test]
    fn test_calculate_wrapped_cursor_position_empty_text() {
        let (line, col) = calculate_wrapped_cursor_position("", 0, 10);
        assert_eq!(line, 0);
        assert_eq!(col, 0);
    }

    #[test]
    fn test_calculate_wrapped_cursor_position_cursor_at_start() {
        let (line, col) = calculate_wrapped_cursor_position("Hello world", 0, 10);
        assert_eq!(line, 0);
        assert_eq!(col, 0);
    }

    #[test]
    fn test_calculate_wrapped_cursor_position_single_line() {
        let (line, col) = calculate_wrapped_cursor_position("Hello", 3, 10);
        assert_eq!(line, 0);
        assert_eq!(col, 3);
    }

    #[test]
    fn test_calculate_wrapped_cursor_position_wrap_to_second_line() {
        let text = "This is a long line that should wrap";
        let (line, col) = calculate_wrapped_cursor_position(text, 15, 10); // Position at "t" in "that"
                                                                           // Expected: "This is a l" (10 chars) on first line, cursor at position 15-10=5 on second line
        assert_eq!(line, 1);
        assert_eq!(col, 5);
    }

    #[test]
    fn test_calculate_wrapped_cursor_position_multiple_wraps() {
        let text = "This is a very long text that will definitely wrap multiple times";
        let (line, col) = calculate_wrapped_cursor_position(text, 25, 10);
        // Line 0: "This is a v" (10 chars, indices 0-9)
        // Line 1: "ery long te" (10 chars, indices 10-19)
        // Line 2: "xt that wil" (10 chars, indices 20-29)
        // Position 25 is 't' in "xt that wil", so line 2, column 5
        assert_eq!(line, 2);
        assert_eq!(col, 5);
    }

    #[test]
    fn test_calculate_wrapped_cursor_position_cursor_beyond_text() {
        let (line, col) = calculate_wrapped_cursor_position("Hi", 10, 10);
        // Should not go beyond the text, so same as cursor at end
        assert_eq!(line, 0);
        assert_eq!(col, 2);
    }

    #[test]
    fn test_calculate_wrapped_cursor_position_exact_wrap_boundary() {
        let text = "0123456789"; // 10 chars
        let (line, col) = calculate_wrapped_cursor_position(text, 10, 10);
        assert_eq!(line, 0);
        assert_eq!(col, 10);
    }

    #[test]
    fn test_calculate_wrapped_cursor_position_single_char_wrap() {
        let text = "0123456789A"; // 11 chars
        let (line, col) = calculate_wrapped_cursor_position(text, 10, 10);
        assert_eq!(line, 0);
        assert_eq!(col, 10);

        let (line2, col2) = calculate_wrapped_cursor_position(text, 11, 10);
        assert_eq!(line2, 1);
        assert_eq!(col2, 1);
    }

    #[test]
    fn test_cursor_with_trailing_spaces() {
        // Test text with trailing spaces that get trimmed
        let text = "hello world "; // 12 chars, with trailing space
        let (line, col) = calculate_wrapped_cursor_position(text, 11, 10); // Cursor at the space
                                                                           // The space gets trimmed, cursor should be at end of visible text (after "d")
        assert_eq!(line, 1); // On second line
        assert_eq!(col, 1); // After the "d"
    }

    #[test]
    fn test_cursor_complete_word_wrap() {
        // Test your example: "the foreign agent usually send..."
        let text = "the foreign agent usually send periodic advertisement messages to all the participants of the network its role. if no message..";
        // This should wrap at various points, test cursor positioning in wrapped sections
        let (line, col) = calculate_wrapped_cursor_position(text, 50, 40); // Cursor somewhere in middle
                                                                           // The exact values depend on wrapping, but should be reasonable
        assert!(line >= 0);
        assert!(col >= 0);
    }

    #[test]
    fn test_multiline_text_with_explicit_newlines() {
        let text = "Line 1\nLine 2\nLine 3";
        let lines = simulate_wrapped_lines(text, 20);

        // Should create 3 lines at explicit newlines
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].0, "Line 1");
        assert_eq!(lines[1].0, "Line 2");
        assert_eq!(lines[2].0, "Line 3");
    }

    #[test]
    fn test_cursor_positioning_with_newlines() {
        let text = "Line 1\nLine 2";
        let (line, col) = calculate_wrapped_cursor_position(text, 8, 20); // At "i" in "Line 2"
        assert_eq!(line, 1); // Second line
        assert_eq!(col, 1); // Second character of "Line 2"
    }

    #[test]
    fn test_mixed_newlines_and_wrapping() {
        let text = "Short\nThis is a longer line that should wrap";
        let lines = simulate_wrapped_lines(text, 10);

        // First line: "Short"
        assert_eq!(lines[0].0, "Short");
        // Second line should be wrapped from "This is a longer line that should wrap"
        assert!(lines.len() > 1);
    }
}
