use unicode_width::UnicodeWidthChar;

/// Convert a byte index to a character index within a string.
/// This handles multi-byte UTF-8 characters correctly.
///
/// # Arguments
/// * `text` - The input string
/// * `byte_pos` - The byte position to convert
///
/// # Returns
/// The character index corresponding to the byte position.
/// Returns the total number of characters if byte_pos is beyond the string length.
pub fn byte_index_to_char_index(text: &str, byte_pos: usize) -> usize {
    if byte_pos >= text.len() {
        return text.chars().count();
    }

    // Find the character that contains the byte at byte_pos
    for (char_index, (byte_idx, ch)) in text.char_indices().enumerate() {
        if byte_idx <= byte_pos && byte_pos < byte_idx + ch.len_utf8() {
            return char_index;
        }
    }

    // Should not reach here if byte_pos is valid
    text.chars().count()
}

pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Simulate how text wraps with trimming (matching ratatui Wrap { trim: true } behavior)
/// Handles both explicit newlines (\n) and automatic wrapping at max_width
/// Returns a vector of (line_text, start_byte_idx, end_byte_idx, start_char_idx, end_char_idx) for each visual line
///
/// Key behaviors matching ratatui:
/// - ALL whitespace between words on same line is preserved
/// - Leading whitespace at start of lines is trimmed
/// - Trailing whitespace at line breaks is trimmed
/// - Wrapping decisions account for actual whitespace width
///
/// Index semantics:
/// - start_byte_idx/start_char_idx: position of first character of first word on the line
/// - end_byte_idx/end_char_idx: position after last character of last word (exclusive),
///   which is also the start position of the next line's content (skipping whitespace)
fn simulate_wrapped_lines(
    text: &str,
    max_width: usize,
) -> Vec<(String, usize, usize, usize, usize)> {
    if text.is_empty() || max_width == 0 {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_line_width: usize = 0;
    let mut line_start_byte_idx: usize = 0;
    let mut line_start_char_idx: usize = 0;
    let mut line_start_set = false;

    // Track pending whitespace between words
    let mut pending_whitespace = String::new();
    let mut pending_whitespace_width: usize = 0;

    let chars: Vec<(usize, usize, char)> = text
        .char_indices()
        .enumerate()
        .map(|(char_idx, (byte_idx, ch))| (char_idx, byte_idx, ch))
        .collect();

    let mut i = 0;
    while i < chars.len() {
        let (char_idx, byte_idx, ch) = chars[i];

        if ch == '\n' {
            // Explicit newline - finalize current line (discard pending whitespace)
            if !current_line.is_empty() {
                lines.push((
                    current_line.clone(),
                    line_start_byte_idx,
                    byte_idx,
                    line_start_char_idx,
                    char_idx,
                ));
            }
            current_line.clear();
            current_line_width = 0;
            pending_whitespace.clear();
            pending_whitespace_width = 0;
            line_start_set = false;
            i += 1;
            continue;
        }

        if ch.is_whitespace() {
            // Accumulate whitespace run
            while i < chars.len() {
                let (_, _, c) = chars[i];
                if !c.is_whitespace() || c == '\n' {
                    break;
                }
                pending_whitespace.push(c);
                pending_whitespace_width += c.width().unwrap_or(1);
                i += 1;
            }
            continue;
        }

        // Found start of a word - extract the complete word
        let word_start_byte = byte_idx;
        let word_start_char = char_idx;
        let mut word_end_byte = byte_idx;

        let mut j = i;
        while j < chars.len() {
            let (_, b_idx, c) = chars[j];
            if c.is_whitespace() || c == '\n' {
                break;
            }
            word_end_byte = b_idx + c.len_utf8();
            j += 1;
        }

        let word = &text[word_start_byte..word_end_byte];
        let word_width: usize = word.chars().map(|c| c.width().unwrap_or(1)).sum();

        // Handle words longer than max_width by breaking them character-by-character
        if word_width > max_width {
            // First, finalize current line if it has content (discard pending whitespace)
            if !current_line.is_empty() {
                lines.push((
                    current_line.clone(),
                    line_start_byte_idx,
                    word_start_byte,
                    line_start_char_idx,
                    word_start_char,
                ));
                current_line.clear();
                current_line_width = 0;
                line_start_set = false;
            }

            // Clear pending whitespace (leading whitespace before long word is trimmed)
            pending_whitespace.clear();
            pending_whitespace_width = 0;

            // Break the long word character by character
            let word_chars = word.char_indices();
            let mut segment_start_byte = word_start_byte;
            let mut segment_start_char = word_start_char;
            let mut segment = String::new();
            let mut segment_width: usize = 0;
            let mut chars_in_segment: usize = 0;

            for (rel_byte_idx, wc) in word_chars {
                let char_width = wc.width().unwrap_or(1);

                if segment_width + char_width > max_width && !segment.is_empty() {
                    // Push current segment as a line
                    let abs_end_byte = word_start_byte + rel_byte_idx;
                    let abs_end_char = segment_start_char + chars_in_segment;
                    lines.push((
                        segment.clone(),
                        segment_start_byte,
                        abs_end_byte,
                        segment_start_char,
                        abs_end_char,
                    ));
                    segment.clear();
                    segment_width = 0;
                    segment_start_byte = abs_end_byte;
                    segment_start_char = abs_end_char;
                    chars_in_segment = 0;
                }

                segment.push(wc);
                segment_width += char_width;
                chars_in_segment += 1;
            }

            // After breaking the long word, the remaining segment becomes current_line
            if !segment.is_empty() {
                current_line = segment;
                current_line_width = segment_width;
                line_start_byte_idx = segment_start_byte;
                line_start_char_idx = segment_start_char;
                line_start_set = true;
            }

            i = j;
            continue;
        }

        // Normal word - check if it fits on current line with pending whitespace
        let space_width = if current_line.is_empty() {
            0 // Leading whitespace - will be trimmed
        } else {
            pending_whitespace_width // Use ACTUAL whitespace width
        };

        if current_line_width + space_width + word_width > max_width && !current_line.is_empty() {
            // Word doesn't fit - finalize current line and start new one
            // Don't add pending whitespace (it becomes trailing whitespace, trimmed)
            lines.push((
                current_line.clone(),
                line_start_byte_idx,
                word_start_byte,
                line_start_char_idx,
                word_start_char,
            ));
            current_line = word.to_string();
            current_line_width = word_width;
            line_start_byte_idx = word_start_byte;
            line_start_char_idx = word_start_char;
            line_start_set = true;

            // Clear pending whitespace (trimmed at wrap boundary)
            pending_whitespace.clear();
            pending_whitespace_width = 0;
        } else {
            // Word fits on current line
            if !line_start_set {
                line_start_byte_idx = word_start_byte;
                line_start_char_idx = word_start_char;
                line_start_set = true;
            }

            // Add pending whitespace if line has content (preserve ALL spaces)
            if !current_line.is_empty() && !pending_whitespace.is_empty() {
                current_line.push_str(&pending_whitespace);
                current_line_width += pending_whitespace_width;
            }

            // Add word
            current_line.push_str(word);
            current_line_width += word_width;

            // Clear pending whitespace
            pending_whitespace.clear();
            pending_whitespace_width = 0;
        }

        i = j;
    }

    // Finalize the last line (discard any trailing pending whitespace)
    if !current_line.is_empty() {
        let text_len = text.len();
        let char_count = text.chars().count();
        lines.push((
            current_line,
            line_start_byte_idx,
            text_len,
            line_start_char_idx,
            char_count,
        ));
    }

    lines
}

/// Calculate the display column for a cursor position within a line.
///
/// This function matches ratatui's `Wrap { trim: true }` behavior:
/// - Leading whitespace at line start is skipped (trimmed)
/// - ALL spaces between words are preserved (matching ratatui exactly)
/// - Trailing whitespace on last line is preserved (cursor advances when typing spaces)
/// - Trailing whitespace at wrap boundaries is trimmed (not counted)
/// - Non-whitespace characters count their actual display width
///
/// # Arguments
/// * `text` - The full text
/// * `line_start_byte` - Byte index where the line starts in the original text
/// * `cursor_byte` - Byte index of the cursor position
/// * `line_end_byte` - Byte index where the line ends (exclusive) - helps detect trailing whitespace
/// * `is_last_line` - Whether this is the last line (trailing spaces preserved) or intermediate line (trimmed at wrap)
///
/// # Returns
/// The display column (0-based) where the cursor should appear
fn calculate_display_column_in_range(
    text: &str,
    line_start_byte: usize,
    cursor_byte: usize,
    line_end_byte: usize,
    is_last_line: bool,
) -> usize {
    if cursor_byte <= line_start_byte || line_start_byte >= text.len() {
        return 0;
    }

    let cursor_end = cursor_byte.min(text.len());
    let line_end = line_end_byte.min(text.len());

    let substr = &text[line_start_byte..cursor_end];

    let mut display_col = 0;
    let mut started = false;
    let mut byte_pos = line_start_byte;

    for ch in substr.chars() {
        if ch == '\n' {
            break;
        } else if ch.is_whitespace() {
            // Check if there's non-whitespace content after THIS character on the line
            let after_this_char = &text[(byte_pos + ch.len_utf8()).min(line_end)..line_end];
            let has_content_after = after_this_char
                .chars()
                .any(|c| !c.is_whitespace() && c != '\n');

            // Count space if:
            // 1. We've seen non-whitespace content (started), AND
            // 2. EITHER: There's more content after (not trailing)
            //    OR: This is the last line (trailing spaces preserved on last line)
            // This matches ratatui: trailing spaces preserved if line has content
            if started && (has_content_after || is_last_line) {
                display_col += 1;
            }
            // Else: skip leading whitespace or trailing at wrap boundaries
        } else {
            // Non-whitespace character
            display_col += ch.width().unwrap_or(1);
            started = true;
        }

        byte_pos += ch.len_utf8();
    }

    display_col
}

/// Calculate the line and column position of a cursor within wrapped text.
/// Accounts for trimming behavior (matching ratatui Wrap { trim: true }).
/// Returns (line_number, column_in_line) for the given cursor position in the text.
///
/// When the cursor is positioned at whitespace that gets trimmed during wrapping,
/// it maps to the end of the previous word (i.e., end of the current visual line).
pub fn calculate_wrapped_cursor_position(
    text: &str,
    cursor_index: usize,
    max_width: usize,
) -> (usize, usize) {
    if text.is_empty() || cursor_index == 0 {
        return (0, 0);
    }

    // Convert cursor byte index to character index for proper multi-byte UTF-8 handling
    let cursor_char_index = byte_index_to_char_index(text, cursor_index);

    // Simulate how the text would be wrapped and trimmed
    let wrapped_lines = simulate_wrapped_lines(text, max_width);

    if wrapped_lines.is_empty() {
        return (0, 0);
    }

    let is_last_line = |idx: usize| idx == wrapped_lines.len() - 1;

    // Find which visual line contains the cursor
    for (line_idx, (_, start_byte_idx, end_byte_idx, start_char_idx, end_char_idx)) in
        wrapped_lines.iter().enumerate()
    {
        // Cursor is within this line's character range
        if cursor_char_index >= *start_char_idx && cursor_char_index < *end_char_idx {
            // Calculate display column accounting for whitespace behavior
            let col_in_line = calculate_display_column_in_range(
                text,
                *start_byte_idx,
                cursor_index,
                *end_byte_idx,
                is_last_line(line_idx),
            );
            return (line_idx, col_in_line);
        }

        // Check if cursor is exactly at end_char_idx (line boundary)
        if cursor_char_index == *end_char_idx {
            if is_last_line(line_idx) {
                // Last line: cursor at end, preserve trailing spaces
                let col_in_line = calculate_display_column_in_range(
                    text,
                    *start_byte_idx,
                    cursor_index,
                    *end_byte_idx,
                    true,
                );
                return (line_idx, col_in_line);
            } else {
                // Not last line: cursor should be at start of next line
                return (line_idx + 1, 0);
            }
        }
    }

    // Cursor is beyond all line ranges - map to end of last line
    if let Some((_, start_byte_idx, end_byte_idx, _, _)) = wrapped_lines.last() {
        let col = calculate_display_column_in_range(
            text,
            *start_byte_idx,
            cursor_index,
            *end_byte_idx,
            true, // Last line
        );
        return (wrapped_lines.len() - 1, col);
    }

    (0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== simulate_wrapped_lines tests ====================

    #[test]
    fn test_empty_input() {
        let lines = simulate_wrapped_lines("", 10);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_zero_width() {
        let lines = simulate_wrapped_lines("Hello", 0);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_single_word_fits() {
        let lines = simulate_wrapped_lines("Hello", 10);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].0, "Hello");
    }

    #[test]
    fn test_two_words_fit_on_one_line() {
        let lines = simulate_wrapped_lines("Hello world", 12);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].0, "Hello world");
    }

    #[test]
    fn test_two_words_wrap_to_two_lines() {
        let lines = simulate_wrapped_lines("Hello world", 10);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].0, "Hello");
        assert_eq!(lines[1].0, "world");
    }

    #[test]
    fn test_multiple_words_wrap() {
        let lines = simulate_wrapped_lines("Hello world test string", 12);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].0, "Hello world");
        assert_eq!(lines[1].0, "test string");
    }

    #[test]
    fn test_exact_fit() {
        // "Hello" is 5 chars, max_width=5 should fit exactly
        let lines = simulate_wrapped_lines("Hello", 5);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].0, "Hello");
    }

    #[test]
    fn test_word_boundary_preservation() {
        // Ensure words are never split unless too long
        let lines = simulate_wrapped_lines("abc defgh", 6);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].0, "abc");
        assert_eq!(lines[1].0, "defgh");
    }

    #[test]
    fn test_long_word_character_break() {
        // Word "abcdefghij" is 10 chars, max_width=5 should break it
        let lines = simulate_wrapped_lines("abcdefghij", 5);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].0, "abcde");
        assert_eq!(lines[1].0, "fghij");
    }

    #[test]
    fn test_long_word_with_other_words() {
        let lines = simulate_wrapped_lines("Hi abcdefghij there", 5);
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0].0, "Hi");
        assert_eq!(lines[1].0, "abcde");
        assert_eq!(lines[2].0, "fghij");
        assert_eq!(lines[3].0, "there");
    }

    #[test]
    fn test_explicit_newline() {
        let lines = simulate_wrapped_lines("Line1\nLine2", 20);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].0, "Line1");
        assert_eq!(lines[1].0, "Line2");
    }

    #[test]
    fn test_multiple_newlines() {
        let lines = simulate_wrapped_lines("A\nB\nC", 10);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].0, "A");
        assert_eq!(lines[1].0, "B");
        assert_eq!(lines[2].0, "C");
    }

    #[test]
    fn test_newline_with_wrapping() {
        let lines = simulate_wrapped_lines("Hello world\ntest string", 10);
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0].0, "Hello");
        assert_eq!(lines[1].0, "world");
        assert_eq!(lines[2].0, "test");
        assert_eq!(lines[3].0, "string");
    }

    #[test]
    fn test_leading_whitespace_trimmed() {
        let lines = simulate_wrapped_lines("   Hello", 10);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].0, "Hello");
    }

    #[test]
    fn test_trailing_whitespace_trimmed() {
        let lines = simulate_wrapped_lines("Hello   ", 10);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].0, "Hello");
    }

    #[test]
    fn test_multiple_spaces_between_words() {
        // All spaces between words are preserved (matching ratatui)
        let lines = simulate_wrapped_lines("Hello    world", 20);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].0, "Hello    world"); // All 4 spaces preserved
    }

    #[test]
    fn test_multibyte_utf8_characters() {
        // Chinese characters are typically 2 display width each
        // "Hello" = 5, space = 1, "世" = 2, "界" = 2 -> total 10, fits on one line with width 10
        let lines = simulate_wrapped_lines("Hello 世界", 10);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].0, "Hello 世界");

        // With width 9, "Hello 世界" (10 width) won't fit, should wrap
        let lines = simulate_wrapped_lines("Hello 世界", 9);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].0, "Hello");
        assert_eq!(lines[1].0, "世界");
    }

    #[test]
    fn test_wrapping_decision_with_multiple_spaces() {
        // "Hello     world" (5 spaces between words)
        // Total width = 5 (Hello) + 5 (spaces) + 5 (world) = 15

        // Width 15: exactly fits
        let lines = simulate_wrapped_lines("Hello     world", 15);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].0, "Hello     world"); // ALL 5 spaces preserved

        // Width 14: doesn't fit, should wrap
        let lines = simulate_wrapped_lines("Hello     world", 14);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].0, "Hello"); // Trailing 5 spaces trimmed at wrap
        assert_eq!(lines[1].0, "world");

        // Width 20: comfortably fits
        let lines = simulate_wrapped_lines("Hello     world", 20);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].0, "Hello     world"); // ALL spaces preserved
    }

    #[test]
    fn test_multiple_spaces_preserved_in_line_text() {
        // Verify line_text actually contains all spaces
        let lines = simulate_wrapped_lines("a  b   c", 20);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].0, "a  b   c"); // 2 spaces, then 3 spaces - all preserved
    }

    #[test]
    fn test_wrapping_with_spaces_before_boundary() {
        // "word  test" (2 spaces), width=9
        // 4 (word) + 2 (spaces) + 4 (test) = 10 > 9 → should wrap
        let lines = simulate_wrapped_lines("word  test", 9);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].0, "word"); // Trailing 2 spaces trimmed at wrap
        assert_eq!(lines[1].0, "test");

        // With width=10, exactly fits
        let lines = simulate_wrapped_lines("word  test", 10);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].0, "word  test"); // Both spaces preserved
    }

    #[test]
    fn test_index_tracking_simple() {
        let text = "Hello world";
        let lines = simulate_wrapped_lines(text, 10);

        // Line 0: "Hello" starts at char 0, ends at char 6 (exclusive, includes space position)
        assert_eq!(lines[0].3, 0); // start_char_idx
        assert_eq!(lines[0].4, 6); // end_char_idx (position of 'w' in original)

        // Line 1: "world" starts at char 6, ends at char 11
        assert_eq!(lines[1].3, 6); // start_char_idx
        assert_eq!(lines[1].4, 11); // end_char_idx
    }

    #[test]
    fn test_index_tracking_with_leading_spaces() {
        let text = "  Hello";
        let lines = simulate_wrapped_lines(text, 10);

        // "Hello" starts at char 2 (after two spaces)
        assert_eq!(lines[0].3, 2);
        assert_eq!(lines[0].4, 7);
    }

    // ==================== calculate_wrapped_cursor_position tests ====================

    #[test]
    fn test_cursor_at_start() {
        let (line, col) = calculate_wrapped_cursor_position("Hello world", 0, 10);
        assert_eq!(line, 0);
        assert_eq!(col, 0);
    }

    #[test]
    fn test_cursor_empty_text() {
        let (line, col) = calculate_wrapped_cursor_position("", 5, 10);
        assert_eq!(line, 0);
        assert_eq!(col, 0);
    }

    #[test]
    fn test_cursor_in_first_word() {
        // "Hel|lo world" - cursor at byte 3
        let (line, col) = calculate_wrapped_cursor_position("Hello world", 3, 10);
        assert_eq!(line, 0);
        assert_eq!(col, 3);
    }

    #[test]
    fn test_cursor_at_end_of_first_word() {
        // "Hello| world" - cursor at byte 5
        let (line, col) = calculate_wrapped_cursor_position("Hello world", 5, 10);
        assert_eq!(line, 0);
        assert_eq!(col, 5);
    }

    #[test]
    fn test_cursor_at_space_between_words() {
        // "Hello |world" - cursor at byte 6 (the space)
        // With wrapping at width 10, "Hello" is line 0, "world" is line 1
        // The space is trimmed, so cursor should map to end of line 0
        let (line, col) = calculate_wrapped_cursor_position("Hello world", 6, 10);
        assert_eq!(line, 1);
        assert_eq!(col, 0);
    }

    #[test]
    fn test_cursor_at_start_of_second_word() {
        // "Hello w|orld" - cursor at byte 7
        let (line, col) = calculate_wrapped_cursor_position("Hello world", 7, 10);
        assert_eq!(line, 1);
        assert_eq!(col, 1);
    }

    #[test]
    fn test_cursor_at_end_of_text() {
        // "Hello world|" - cursor at byte 11 (end)
        let (line, col) = calculate_wrapped_cursor_position("Hello world", 11, 10);
        assert_eq!(line, 1);
        assert_eq!(col, 5);
    }

    #[test]
    fn test_cursor_beyond_text() {
        // Cursor at byte 20, but text is only 11 bytes
        let (line, col) = calculate_wrapped_cursor_position("Hello world", 20, 10);
        assert_eq!(line, 1);
        assert_eq!(col, 5);
    }

    #[test]
    fn test_cursor_with_no_wrap_needed() {
        // Text fits on one line
        let (line, col) = calculate_wrapped_cursor_position("Hello", 3, 20);
        assert_eq!(line, 0);
        assert_eq!(col, 3);
    }

    #[test]
    fn test_cursor_with_newline() {
        // "Line1\nLine2" with cursor at "L" of "Line2"
        let text = "Line1\nLine2";
        let cursor_byte = 6; // Position of 'L' in "Line2"
        let (line, col) = calculate_wrapped_cursor_position(text, cursor_byte, 20);
        assert_eq!(line, 1);
        assert_eq!(col, 0);
    }

    #[test]
    fn test_cursor_in_second_line_after_newline() {
        let text = "Line1\nLine2";
        let cursor_byte = 8; // Position of 'n' in "Line2"
        let (line, col) = calculate_wrapped_cursor_position(text, cursor_byte, 20);
        assert_eq!(line, 1);
        assert_eq!(col, 2);
    }

    #[test]
    fn test_cursor_with_multibyte_chars() {
        // "Hello 世界" - Chinese chars are 3 bytes each in UTF-8
        let text = "Hello 世界";
        // "Hello" = 5 bytes, space = 1 byte, "世" = 3 bytes, "界" = 3 bytes = 12 bytes total
        // Display width: "Hello" = 5, space = 1, "世" = 2, "界" = 2 = 10 total

        // With width 10, everything fits on one line
        // Cursor at "世" (byte 6, char 6)
        let (line, col) = calculate_wrapped_cursor_position(text, 6, 10);
        assert_eq!(line, 0);
        assert_eq!(col, 6);

        // With width 9, "Hello 世界" (10 width) wraps: "Hello" on line 0, "世界" on line 1
        let (line, col) = calculate_wrapped_cursor_position(text, 6, 9);
        // Cursor at byte 6 = char 6 = first char of "世界" = line 1, col 0
        assert_eq!(line, 1);
        assert_eq!(col, 0);
    }

    #[test]
    fn test_cursor_in_long_word_break() {
        // "abcdefghij" with width 5 breaks into "abcde" and "fghij"
        let text = "abcdefghij";

        // Cursor at 'c' (byte 2)
        let (line, col) = calculate_wrapped_cursor_position(text, 2, 5);
        assert_eq!(line, 0);
        assert_eq!(col, 2);

        // Cursor at 'f' (byte 5)
        let (line, col) = calculate_wrapped_cursor_position(text, 5, 5);
        assert_eq!(line, 1);
        assert_eq!(col, 0);

        // Cursor at 'h' (byte 7)
        let (line, col) = calculate_wrapped_cursor_position(text, 7, 5);
        assert_eq!(line, 1);
        assert_eq!(col, 2);
    }

    #[test]
    fn test_cursor_trailing_spaces() {
        // "Hello   " with trailing spaces
        let text = "Hello   ";
        // Cursor at end (byte 8, char 8, after all 3 trailing spaces)
        let (line, col) = calculate_wrapped_cursor_position(text, 8, 10);
        // Trailing spaces on last line are preserved (ratatui behavior)
        // User should see cursor advance when typing spaces
        assert_eq!(line, 0);
        assert_eq!(col, 8); // Hello(5) + 3 spaces = 8
    }

    #[test]
    fn test_cursor_single_trailing_space() {
        // "Hello " with single trailing space
        let text = "Hello ";
        // Cursor at byte 6 (after the single trailing space)
        let (line, col) = calculate_wrapped_cursor_position(text, 6, 10);
        // Trailing space on last line is preserved
        assert_eq!(line, 0);
        assert_eq!(col, 6); // Hello(5) + 1 space = 6
    }

    #[test]
    fn test_cursor_leading_spaces() {
        // "   Hello" with leading spaces
        let text = "   Hello";
        // Cursor at 'H' (byte 3)
        let (line, col) = calculate_wrapped_cursor_position(text, 3, 10);
        assert_eq!(line, 0);
        assert_eq!(col, 0); // "Hello" starts at display col 0 due to trimming
    }

    #[test]
    fn test_cursor_in_multiple_spaces_between_words() {
        // "word     another" with 5 spaces between words
        // Ratatui preserves ALL spaces between words on same line
        // Display: "word     another" (all 5 spaces visible)
        let text = "word     another";

        // Cursor at position 4 (end of "word", before spaces)
        let (line, col) = calculate_wrapped_cursor_position(text, 4, 20);
        assert_eq!(line, 0);
        assert_eq!(col, 4); // w(1) + o(1) + r(1) + d(1) = 4

        // Cursor at position 5 (first space after "word")
        let (line, col) = calculate_wrapped_cursor_position(text, 5, 20);
        assert_eq!(line, 0);
        assert_eq!(col, 5); // word(4) + space(1) = 5

        // Cursor at position 6 (2nd space)
        let (line, col) = calculate_wrapped_cursor_position(text, 6, 20);
        assert_eq!(line, 0);
        assert_eq!(col, 6); // word(4) + 2 spaces = 6

        // Cursor at position 7 (3rd space of 5)
        let (line, col) = calculate_wrapped_cursor_position(text, 7, 20);
        assert_eq!(line, 0);
        assert_eq!(col, 7); // word(4) + 3 spaces = 7

        // Cursor at position 9 (5th/last space before "another")
        let (line, col) = calculate_wrapped_cursor_position(text, 9, 20);
        assert_eq!(line, 0);
        assert_eq!(col, 9); // word(4) + 5 spaces = 9

        // Cursor at position 10 ('a' in "another")
        let (line, col) = calculate_wrapped_cursor_position(text, 10, 20);
        assert_eq!(line, 0);
        assert_eq!(col, 10); // word(4) + 5 spaces + a(1) = 10
    }

    #[test]
    fn test_cursor_multiple_spaces_with_wrapping() {
        // "Hello     world" with 5 spaces, width=10 forces wrap
        let text = "Hello     world";

        // With width 10, "Hello" fits on line 0, "world" wraps to line 1

        // Cursor at position 5 (first space after "Hello")
        let (line, col) = calculate_wrapped_cursor_position(text, 5, 10);
        assert_eq!(line, 0);
        assert_eq!(col, 5); // End of "Hello" (trailing spaces trimmed at wrap)

        // Cursor at position 7 (middle of the 5 spaces)
        let (line, col) = calculate_wrapped_cursor_position(text, 7, 10);
        assert_eq!(line, 0);
        assert_eq!(col, 5); // Still end of "Hello"

        // Cursor at position 10 ('w' in "world")
        let (line, col) = calculate_wrapped_cursor_position(text, 10, 10);
        assert_eq!(line, 1);
        assert_eq!(col, 0); // Start of "world" on next line
    }

    #[test]
    fn test_cursor_only_spaces() {
        // Text with only spaces
        let text = "          ";

        // Cursor at position 5
        let (line, col) = calculate_wrapped_cursor_position(text, 5, 20);
        assert_eq!(line, 0);
        assert_eq!(col, 0); // All spaces trimmed, cursor at position 0
    }

    #[test]
    fn test_cursor_multiple_spaces_no_wrapping() {
        // "Hello     world" with 5 spaces, width=20 (no wrapping)
        // All spaces should be preserved since they're on same line
        let text = "Hello     world";

        // Cursor at position 5 (first space after "Hello")
        let (line, col) = calculate_wrapped_cursor_position(text, 5, 20);
        assert_eq!(line, 0);
        assert_eq!(col, 5); // Hello(5) + space(1) = 5... wait, that's the space itself

        // Cursor at position 6 (2nd space)
        let (line, col) = calculate_wrapped_cursor_position(text, 6, 20);
        assert_eq!(line, 0);
        assert_eq!(col, 6); // Hello(5) + 2 spaces = 6

        // Cursor at position 9 (5th space, last before "world")
        let (line, col) = calculate_wrapped_cursor_position(text, 9, 20);
        assert_eq!(line, 0);
        assert_eq!(col, 9); // Hello(5) + 5 spaces = 9

        // Cursor at position 10 ('w' in "world")
        let (line, col) = calculate_wrapped_cursor_position(text, 10, 20);
        assert_eq!(line, 0);
        assert_eq!(col, 10); // Hello(5) + 5 spaces + w(1) = 10

        // Cursor at position 15 (end of "world")
        let (line, col) = calculate_wrapped_cursor_position(text, 15, 20);
        assert_eq!(line, 0);
        assert_eq!(col, 15); // Hello(5) + 5 spaces + world(5) = 15
    }

    // ==================== byte_index_to_char_index tests ====================

    #[test]
    fn test_byte_to_char_ascii() {
        let text = "Hello";
        assert_eq!(byte_index_to_char_index(text, 0), 0);
        assert_eq!(byte_index_to_char_index(text, 2), 2);
        assert_eq!(byte_index_to_char_index(text, 5), 5);
    }

    #[test]
    fn test_byte_to_char_multibyte() {
        let text = "世界"; // Each character is 3 bytes
        assert_eq!(byte_index_to_char_index(text, 0), 0); // First byte of "世"
        assert_eq!(byte_index_to_char_index(text, 1), 0); // Second byte of "世"
        assert_eq!(byte_index_to_char_index(text, 2), 0); // Third byte of "世"
        assert_eq!(byte_index_to_char_index(text, 3), 1); // First byte of "界"
        assert_eq!(byte_index_to_char_index(text, 6), 2); // Beyond end
    }

    #[test]
    fn test_byte_to_char_mixed() {
        let text = "Hi世界"; // "Hi" = 2 bytes, "世界" = 6 bytes
        assert_eq!(byte_index_to_char_index(text, 0), 0); // 'H'
        assert_eq!(byte_index_to_char_index(text, 1), 1); // 'i'
        assert_eq!(byte_index_to_char_index(text, 2), 2); // "世"
        assert_eq!(byte_index_to_char_index(text, 5), 3); // "界"
    }
}
