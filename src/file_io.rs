use std::fs;
use std::io::{self, Seek, SeekFrom, Write};
use std::time::UNIX_EPOCH;

use serde_json;

use crate::ai::AIFeedback;

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_len = 0;

    for word in text.split_whitespace() {
        let word_len = word.len();

        if !current_line.is_empty() {
            current_line.push(' ');
            current_len += 1;
        }

        if current_len + word_len <= max_width {
            current_line.push_str(word);
            current_len += word_len;
        } else {
            result.push(current_line);
            current_line = word.to_string();
            current_len = word_len;
        }
    }

    if !current_line.is_empty() {
        result.push(current_line);
    }

    result
}

pub fn write_session_header(
    file: &mut fs::File,
    deck_name: &str,
    total_questions: usize,
) -> io::Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    writeln!(
        file,
        "======================================================================"
    )?;
    writeln!(file, "QUIZ SESSION: {}", deck_name)?;
    writeln!(file, "Started: {}", timestamp)?;
    writeln!(
        file,
        "======================================================================"
    )?;
    writeln!(file)?;
    writeln!(file, "Progress: 0/{} questions answered", total_questions)?;
    writeln!(
        file,
        "======================================================================"
    )?;
    writeln!(file)?;

    Ok(())
}

pub fn update_progress_header(
    file: &mut fs::File,
    answered: usize,
    total: usize,
) -> io::Result<()> {
    let current_pos = file.stream_position()?;
    file.seek(SeekFrom::Start(current_pos.saturating_sub(100)))?;
    writeln!(file, "Progress: {}/{} questions answered", answered, total)?;
    writeln!(
        file,
        "======================================================================"
    )?;
    writeln!(file)?;
    Ok(())
}

pub fn write_question_entry(
    file: &mut fs::File,
    question_num: usize,
    question: &str,
    user_answer: &Option<String>,
    correct_answer: &str,
    ai_feedback: Option<&AIFeedback>,
) -> io::Result<()> {
    let user_ans_text = user_answer
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("[No answer]");

    writeln!(file, "QUESTION {}:", question_num)?;
    for line in wrap_text(question, 88) {
        writeln!(file, "{}", line)?;
    }
    writeln!(file)?;

    writeln!(file, "YOUR ANSWER:")?;
    for line in wrap_text(user_ans_text, 88) {
        writeln!(file, "{}", line)?;
    }
    writeln!(file)?;

    writeln!(file, "CORRECT ANSWER:")?;
    for line in wrap_text(correct_answer, 88) {
        writeln!(file, "{}", line)?;
    }
    writeln!(file)?;

    if let Some(feedback) = ai_feedback {
        writeln!(file, "AI FEEDBACK:")?;
        let json = serde_json::to_string_pretty(feedback)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        for line in json.lines() {
            writeln!(file, "{}", line)?;
        }
        writeln!(file)?;
    }

    writeln!(
        file,
        "-----------------------------------------------------------------------"
    )?;
    writeln!(file)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{write_question_entry, AIFeedback};

    #[test]
    fn test_input_buffer_operations() {
        let mut buffer = String::new();
        buffer.push('H');
        buffer.push('i');
        assert_eq!(buffer, "Hi");
        buffer.pop();
        assert_eq!(buffer, "H");
        assert!(buffer.trim().is_empty() == false);
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

    #[test]
    fn test_write_question_entry_with_ai_feedback() {
        use std::fs::File;
        use std::io::Read;

        let temp_path = std::env::temp_dir().join("test_ai_feedback.txt");
        let mut file = File::create(&temp_path).unwrap();

        let ai_feedback = AIFeedback {
            is_correct: false,
            correctness_score: 0.75,
            corrections: vec!["Missed key point".to_string()],
            explanation: "Your answer was close but missed the main concept.".to_string(),
            suggestions: vec!["Review chapter 5".to_string()],
        };

        write_question_entry(
            &mut file,
            1,
            "What is the capital of France?",
            &Some("Paris is the capital".to_string()),
            "Paris",
            Some(&ai_feedback),
        )
        .unwrap();

        let mut content = String::new();
        File::open(&temp_path)
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        std::fs::remove_file(temp_path).unwrap();

        assert!(content.contains("QUESTION 1:"));
        assert!(content.contains("YOUR ANSWER:"));
        assert!(content.contains("Paris is the capital"));
        assert!(content.contains("CORRECT ANSWER:"));
        assert!(content.contains("Paris"));
        assert!(content.contains("AI FEEDBACK:"));
        assert!(content.contains("\"is_correct\": false"));
        assert!(content.contains("\"correctness_score\": 0.75"));
        assert!(content
            .contains("\"explanation\": \"Your answer was close but missed the main concept.\""));
        assert!(content.contains("\"corrections\": [\n    \"Missed key point\"\n  ]"));
        assert!(content.contains("\"suggestions\": [\n    \"Review chapter 5\"\n  ]"));
    }

    #[test]
    fn test_write_question_entry_without_ai_feedback() {
        use std::fs::File;
        use std::io::Read;

        let temp_path = std::env::temp_dir().join("test_no_ai_feedback.txt");
        let mut file = File::create(&temp_path).unwrap();

        write_question_entry(
            &mut file,
            2,
            "What is 2+2?",
            &Some("4".to_string()),
            "4",
            None,
        )
        .unwrap();

        let mut content = String::new();
        File::open(&temp_path)
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        std::fs::remove_file(temp_path).unwrap();

        assert!(content.contains("QUESTION 2:"));
        assert!(content.contains("YOUR ANSWER:"));
        assert!(content.contains("4"));
        assert!(content.contains("CORRECT ANSWER:"));
        assert!(content.contains("4"));
        assert!(!content.contains("AI FEEDBACK:"));
    }
}
