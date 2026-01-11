pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Calculate the line and column position of a cursor within wrapped text.
/// Returns (line_number, column_in_line) for the given cursor position in the text.
pub fn calculate_wrapped_cursor_position(
    text: &str,
    cursor_index: usize,
    max_width: usize,
) -> (usize, usize) {
    if text.is_empty() || cursor_index == 0 {
        return (0, 0);
    }

    let mut line = 0;
    let mut col = 0;
    let mut current_line_width = 0;

    for (byte_idx, ch) in text.char_indices() {
        if byte_idx >= cursor_index {
            break;
        }

        let char_width = ch.len_utf8(); // Simplified: using byte length as width

        if current_line_width + char_width > max_width && current_line_width > 0 {
            // Wrap to next line
            line += 1;
            col = 0;
            current_line_width = char_width;
        } else {
            current_line_width += char_width;
        }
        col += char_width;
    }

    (line, col)
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
}
