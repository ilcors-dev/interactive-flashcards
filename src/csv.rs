use crate::{db, models::Flashcard};
use std::fs;
use std::path::PathBuf;

pub fn get_flashcards_dir() -> PathBuf {
    db::get_data_dir().join("flashcards")
}

pub fn get_csv_files() -> Vec<PathBuf> {
    let flashcards_dir = get_flashcards_dir();
    let mut files = Vec::new();

    if !flashcards_dir.exists() {
        let _ = fs::create_dir_all(&flashcards_dir);
    }

    if flashcards_dir.exists() && flashcards_dir.is_dir()
        && let Ok(entries) = fs::read_dir(&flashcards_dir)
    {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension()
                && ext == "csv"
            {
                files.push(entry.path());
            }
        }
    }

    files.sort();
    files
}

pub fn load_csv(path: &PathBuf) -> std::io::Result<Vec<Flashcard>> {
    let content = fs::read_to_string(path)?;
    let mut flashcards = Vec::new();

    for line in content.lines() {
        if let Some((question, answer)) = parse_csv_line(line)
            && !question.trim().is_empty() && !answer.trim().is_empty() {
                flashcards.push(Flashcard {
                    question,
                    answer,
                    user_answer: None,
                    ai_feedback: None,
                    written_to_file: false,
                    id: None,
                });
            }
    }

    Ok(flashcards)
}

pub fn parse_csv_line(line: &str) -> Option<(String, String)> {
    let mut chars = line.chars().peekable();
    let mut question = String::new();
    let mut answer = String::new();
    let mut current_field = &mut question;
    let mut in_quotes = false;
    let mut field_index = 0;

    while let Some(c) = chars.next() {
        match c {
            '"' if !in_quotes => {
                in_quotes = true;
            }
            '"' if in_quotes => {
                if chars.peek() == Some(&',') {
                    chars.next();
                    in_quotes = false;
                    if field_index == 0 {
                        current_field = &mut answer;
                        field_index = 1;
                    }
                } else if chars.peek() == Some(&'"') {
                    chars.next();
                    current_field.push('"');
                } else {
                    in_quotes = false;
                    if field_index == 0 {
                        current_field = &mut answer;
                        field_index = 1;
                    }
                }
            }
            ',' if !in_quotes && field_index == 0 => {
                field_index = 1;
                current_field = &mut answer;
            }
            _ => {
                current_field.push(c);
            }
        }
    }

    Some((question, answer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn test_parse_csv_simple() {
        let line = "What is 2+2?,Four";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "What is 2+2?");
        assert_eq!(answer, "Four");
    }

    #[test]
    fn test_parse_csv_with_quotes() {
        let line = "\"What is 2+2?\",\"Four\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "What is 2+2?");
        assert_eq!(answer, "Four");
    }

    #[test]
    fn test_parse_csv_with_commas_in_answer() {
        let line = "\"What is 2+2?\",\"Four, or 4\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "What is 2+2?");
        assert_eq!(answer, "Four, or 4");
    }

    #[test]
    fn test_parse_csv_with_commas_in_question() {
        let line = "\"What is 2+2, 3+3?\",\"Four\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "What is 2+2, 3+3?");
        assert_eq!(answer, "Four");
    }

    #[test]
    fn test_parse_csv_with_escaped_quotes() {
        let line = "\"What is \"\"quoted\"\"?\",\"Answer with \"\"quotes\"\"\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "What is \"quoted\"?");
        assert_eq!(answer, "Answer with \"quotes\"");
    }

    #[test]
    fn test_parse_csv_empty_fields() {
        let line = ",";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "");
        assert_eq!(answer, "");
    }

    #[test]
    fn test_parse_csv_complex_example() {
        let line = "\"In a CSV, what does a comma do?\",\"It separates fields, but can be part of a field if quoted\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "In a CSV, what does a comma do?");
        assert_eq!(
            answer,
            "It separates fields, but can be part of a field if quoted"
        );
    }

    #[test]
    fn test_parse_csv_only_question_quoted() {
        let line = "\"What is 2+2?\",Four";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "What is 2+2?");
        assert_eq!(answer, "Four");
    }

    #[test]
    fn test_parse_csv_only_answer_quoted() {
        let line = "What is 2+2?,\"Four\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "What is 2+2?");
        assert_eq!(answer, "Four");
    }

    #[test]
    fn test_parse_csv_line_with_newlines_in_quoted_field() {
        let line = "\"Question\",\"Answer with, comma\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "Question");
        assert_eq!(answer, "Answer with, comma");
    }

    #[test]
    fn test_parse_csv_real_world_example() {
        let line = "\"What is the defining characteristic of a MANET?\",\"It is an infrastructure-less network where all nodes are potentially mobile and communicate directly with each other.\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "What is the defining characteristic of a MANET?");
        assert_eq!(
            answer,
            "It is an infrastructure-less network where all nodes are potentially mobile and communicate directly with each other."
        );
    }

    #[test]
    fn test_parse_csv_multiple_quotes() {
        let line = "\"Is \"\"quoted\"\" text supported?\",\"Yes, \"\"it works\"\" correctly\"";
        let result = parse_csv_line(line);
        assert!(result.is_some());
        let (question, answer) = result.unwrap();
        assert_eq!(question, "Is \"quoted\" text supported?");
        assert_eq!(answer, "Yes, \"it works\" correctly");
    }

    #[test]
    fn test_load_csv_with_empty_lines() {
        let content = "Q1,A1\n\nQ2,A2\n\nQ3,A3";
        let mut flashcards = Vec::new();

        for line in content.lines() {
            if let Some((question, answer)) = parse_csv_line(line) {
                if !question.trim().is_empty() && !answer.trim().is_empty() {
                    flashcards.push(Flashcard {
                        question,
                        answer,
                        user_answer: None,
                        ai_feedback: None,
                        written_to_file: false,
                        id: None,
                    });
                }
            }
        }

        assert_eq!(flashcards.len(), 3);
        assert_eq!(flashcards[0].question, "Q1");
        assert_eq!(flashcards[1].question, "Q2");
        assert_eq!(flashcards[2].question, "Q3");
    }

    #[test]
    fn test_load_csv_filters_empty_fields() {
        let content = "Q1,A1\n,A2\nQ2,\n,Q3\n";
        let mut flashcards = Vec::new();

        for line in content.lines() {
            if let Some((question, answer)) = parse_csv_line(line) {
                if !question.trim().is_empty() && !answer.trim().is_empty() {
                    flashcards.push(Flashcard {
                        question,
                        answer,
                        user_answer: None,
                        ai_feedback: None,
                        written_to_file: false,
                        id: None,
                    });
                }
            }
        }

        assert_eq!(flashcards.len(), 1);
        assert_eq!(flashcards[0].question, "Q1");
        assert_eq!(flashcards[0].answer, "A1");
    }

    #[test]
    fn test_get_flashcards_dir_uses_data_dir() {
        let _lock = env_lock();
        let temp_dir = tempfile::tempdir().unwrap();
        let original_home = env::var("HOME").ok();
        unsafe {
            env::set_var("HOME", temp_dir.path());
        }

        let dir = get_flashcards_dir();

        if let Some(value) = original_home {
            unsafe {
                env::set_var("HOME", value);
            }
        } else {
            unsafe {
                env::remove_var("HOME");
            }
        }

        assert!(dir.starts_with(temp_dir.path()));
        assert!(dir.ends_with(".local/share/interactive-flashcards/flashcards"));
    }

    #[test]
    fn test_get_csv_files_reads_from_data_dir() {
        let _lock = env_lock();
        let temp_dir = tempfile::tempdir().unwrap();
        let original_home = env::var("HOME").ok();
        unsafe {
            env::set_var("HOME", temp_dir.path());
        }

        let flashcards_dir = get_flashcards_dir();
        fs::create_dir_all(&flashcards_dir).unwrap();
        fs::write(flashcards_dir.join("b.csv"), "Q1,A1").unwrap();
        fs::write(flashcards_dir.join("a.csv"), "Q2,A2").unwrap();
        fs::write(flashcards_dir.join("notes.txt"), "ignore").unwrap();

        let files = get_csv_files();

        if let Some(value) = original_home {
            unsafe {
                env::set_var("HOME", value);
            }
        } else {
            unsafe {
                env::remove_var("HOME");
            }
        }

        let expected = vec![flashcards_dir.join("a.csv"), flashcards_dir.join("b.csv")];
        assert_eq!(files, expected);
    }
}
