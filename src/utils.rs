pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
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
}
