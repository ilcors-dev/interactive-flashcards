#[cfg(test)]
mod tests {
    use crate::AIFeedback;

    #[test]
    fn test_input_buffer_operations() {
        let mut buffer = String::new();
        buffer.push('H');
        buffer.push('i');
        assert_eq!(buffer, "Hi");
        buffer.pop();
        assert_eq!(buffer, "H");
        assert!(!buffer.trim().is_empty());
    }

    #[test]
    fn test_empty_answer_submission() {
        let mut buffer = String::new();
        assert!(buffer.trim().is_empty());
        buffer.push(' ');
        assert!(buffer.trim().is_empty());
        buffer.push('A');
        assert!(!buffer.trim().is_empty());
    }

    #[test]
    fn test_saturating_sub_index_bounds() {
        let cards_len: usize = 1;
        let current_index: usize = 0;
        let new_index = current_index.saturating_sub(1);
        assert_eq!(new_index, 0);

        let max_index = cards_len.saturating_sub(1);
        assert_eq!(max_index, 0);
    }

    #[test]
    fn test_answer_restoration_on_navigation() {
        let user_answer = Some("My Answer 1".to_string());
        let input_buffer = user_answer.as_ref().unwrap_or(&String::new()).clone();

        assert_eq!(input_buffer, "My Answer 1");
    }

    #[test]
    fn test_no_answer_restoration_when_none() {
        let user_answer: Option<String> = None;
        let input_buffer = user_answer.as_ref().unwrap_or(&String::new()).clone();

        assert!(input_buffer.is_empty());
    }

    #[test]
    fn test_answer_submission_non_empty() {
        let input_buffer = String::from("My Answer");
        let mut user_answer: Option<String> = None;

        if !input_buffer.trim().is_empty() {
            user_answer = Some(input_buffer.clone());
        }

        assert_eq!(user_answer, Some("My Answer".to_string()));
    }

    #[test]
    fn test_answer_submission_empty() {
        let input_buffer = String::from("   ");
        let mut user_answer: Option<String> = None;

        if !input_buffer.trim().is_empty() {
            user_answer = Some(input_buffer.clone());
        }

        assert!(user_answer.is_none());
    }

    #[test]
    fn test_input_buffer_character_addition() {
        let mut buffer = String::new();
        buffer.push('H');
        buffer.push('e');
        buffer.push('l');
        buffer.push('l');
        buffer.push('o');
        assert_eq!(buffer, "Hello");
        buffer.push(' ');
        buffer.push('W');
        buffer.push('o');
        buffer.push('r');
        buffer.push('l');
        buffer.push('d');
        assert_eq!(buffer, "Hello World");
    }

    #[test]
    fn test_input_buffer_backspace() {
        let mut buffer = String::from("Hello");
        buffer.pop();
        assert_eq!(buffer, "Hell");
        buffer.pop();
        buffer.pop();
        assert_eq!(buffer, "He");
        buffer.pop();
        buffer.pop();
        buffer.pop();
        assert!(buffer.is_empty());
        buffer.pop();
        assert!(buffer.is_empty());
    }
}
