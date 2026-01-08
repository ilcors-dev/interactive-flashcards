use crate::file_io::{update_progress_header, write_question_entry};
use crate::models::{AppState, QuizSession};
use crossterm::event::KeyCode;
use std::io;

pub fn handle_quiz_input(
    session: &mut QuizSession,
    key: KeyCode,
    app_state: &mut AppState,
) -> io::Result<()> {
    if !session.showing_answer {
        match key {
            KeyCode::Esc => {
                *app_state = AppState::QuizQuitConfirm;
                Ok(())
            }
            KeyCode::Down => {
                if session.current_index < session.flashcards.len().saturating_sub(1) {
                    session.current_index += 1;
                    session.showing_answer = false;
                    session.input_buffer = session.flashcards[session.current_index]
                        .user_answer
                        .as_ref()
                        .unwrap_or(&String::new())
                        .clone();
                }
                Ok(())
            }
            KeyCode::Up => {
                if session.current_index > 0 {
                    session.current_index -= 1;
                    session.showing_answer = false;
                    session.input_buffer = session.flashcards[session.current_index]
                        .user_answer
                        .as_ref()
                        .unwrap_or(&String::new())
                        .clone();
                }
                Ok(())
            }
            KeyCode::Enter => {
                if !session.input_buffer.trim().is_empty() {
                    session.flashcards[session.current_index].user_answer =
                        Some(session.input_buffer.clone());

                    session.questions_answered += 1;

                    if let Some(ref mut file) = session.output_file {
                        let q_num = session.current_index + 1;
                        let question = &session.flashcards[session.current_index].question;
                        let user_ans = &session.flashcards[session.current_index].user_answer;
                        let correct_ans = &session.flashcards[session.current_index].answer;

                        write_question_entry(file, q_num, question, user_ans, correct_ans)?;
                        update_progress_header(
                            file,
                            session.questions_answered,
                            session.questions_total,
                        )?;
                    }
                }

                session.input_buffer.clear();
                session.showing_answer = true;
                Ok(())
            }
            KeyCode::Backspace => {
                session.input_buffer.pop();
                Ok(())
            }
            KeyCode::Char(c) => {
                session.input_buffer.push(c);
                Ok(())
            }
            _ => Ok(()),
        }
    } else {
        match key {
            KeyCode::Esc => {
                *app_state = AppState::QuizQuitConfirm;
                Ok(())
            }
            KeyCode::Down => {
                if session.current_index < session.flashcards.len().saturating_sub(1) {
                    session.current_index += 1;
                    session.showing_answer = false;
                }
                Ok(())
            }
            KeyCode::Up => {
                if session.current_index > 0 {
                    session.current_index -= 1;
                    session.showing_answer = false;
                }
                Ok(())
            }
            KeyCode::Enter => {
                if session.current_index < session.flashcards.len().saturating_sub(1) {
                    session.current_index += 1;
                    session.showing_answer = false;
                } else {
                    *app_state = AppState::Summary;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
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
}
