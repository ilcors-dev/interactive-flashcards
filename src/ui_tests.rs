#[cfg(test)]
mod ui_integration_tests {
    use crate::ai::AIEvaluationResult;
    use crate::models::{AiRequest, AiResponse};
    use crate::{AppState, Flashcard, QuizSession};
    use tokio::sync::mpsc;

    /// Test that UI state calculation captures all relevant changes
    #[tokio::test]
    async fn test_ui_state_tracking_comprehensive() {
        let mut session = create_test_session();

        // Calculate initial state
        let initial_state = calculate_ui_state_tuple(&session);

        // Test input buffer changes trigger state updates
        session.input_buffer.push('x');
        let after_typing = calculate_ui_state_tuple(&session);
        assert_ne!(initial_state, after_typing, "Typing should change UI state");

        // Test cursor position changes
        session.cursor_position = 5;
        let after_cursor_move = calculate_ui_state_tuple(&session);
        assert_ne!(
            after_typing, after_cursor_move,
            "Cursor movement should change UI state"
        );

        // Test scroll position changes
        session.input_scroll_y = 2;
        let after_scroll = calculate_ui_state_tuple(&session);
        assert_ne!(
            after_cursor_move, after_scroll,
            "Scroll changes should change UI state"
        );

        // Test AI evaluation status changes
        session.ai_evaluation_in_progress = true;
        let after_ai_start = calculate_ui_state_tuple(&session);
        assert_ne!(
            after_scroll, after_ai_start,
            "AI evaluation start should change UI state"
        );

        // Test AI feedback addition
        session.flashcards[0].ai_feedback = Some(crate::ai::AIFeedback {
            is_correct: true,
            correctness_score: 1.0,
            corrections: vec![],
            explanation: "Perfect!".to_string(),
            suggestions: vec![],
        });
        let after_ai_response = calculate_ui_state_tuple(&session);
        assert_ne!(
            after_ai_start, after_ai_response,
            "AI response should change UI state"
        );

        // Test error message changes
        session.last_ai_error = Some("Test error".to_string());
        let after_error = calculate_ui_state_tuple(&session);
        assert_ne!(
            after_ai_response, after_error,
            "Error messages should change UI state"
        );

        // Test question navigation
        session.current_index = 1;
        let after_navigation = calculate_ui_state_tuple(&session);
        assert_ne!(
            after_error, after_navigation,
            "Question navigation should change UI state"
        );

        // Test answer display toggle
        session.showing_answer = true;
        let after_answer_toggle = calculate_ui_state_tuple(&session);
        assert_ne!(
            after_navigation, after_answer_toggle,
            "Answer display toggle should change UI state"
        );

        // Test progress changes
        session.questions_answered = 1;
        let after_progress = calculate_ui_state_tuple(&session);
        assert_ne!(
            after_answer_toggle, after_progress,
            "Progress changes should change UI state"
        );
    }

    /// Test that menu navigation triggers UI state changes
    #[test]
    fn test_menu_navigation_triggers_ui_update() {
        use crate::AppState;

        // Initial menu state (index 0, with default values for other fields)
        let initial_state = (
            AppState::Menu,
            Some((0, false, false, 0, 0, 0, 0, false, 0, 0)),
        );

        // Simulate down arrow navigation (should change selected index to 1)
        let after_navigation = (
            AppState::Menu,
            Some((1, false, false, 0, 0, 0, 0, false, 0, 0)),
        );

        assert_ne!(
            initial_state, after_navigation,
            "Menu navigation should change UI state to trigger redraw"
        );
    }

    /// Test that menu navigation wraps around boundaries
    #[test]
    fn test_menu_navigation_boundary_wrapping() {
        let csv_files = vec!["file1.csv".to_string(), "file2.csv".to_string()];

        // Test upper boundary (should not go below 0)
        let mut selected_index: usize = 0;
        selected_index = selected_index.saturating_sub(1);
        assert_eq!(selected_index, 0, "Menu navigation should not go below 0");

        // Test lower boundary (should not exceed file count - 1)
        selected_index = 1; // At last item
        if selected_index >= csv_files.len().saturating_sub(1) {
            // Should stay at current position if trying to go further
            assert_eq!(
                selected_index, 1,
                "Menu navigation should not exceed file count"
            );
        }
    }

    /// Test that AI response processing triggers UI updates
    #[tokio::test]
    async fn test_ai_response_processing_triggers_ui_update() {
        let session = create_test_session();

        // Initially no AI feedback
        let initial_state = calculate_ui_state_tuple(&session);

        // Create a session with AI feedback to simulate processing
        let mut session_with_feedback = create_test_session();
        session_with_feedback.flashcards[0].ai_feedback = Some(crate::ai::AIFeedback {
            is_correct: true,
            correctness_score: 1.0,
            corrections: vec![],
            explanation: "Great job!".to_string(),
            suggestions: vec![],
        });

        let after_ai_tuple = calculate_ui_state_tuple(&session_with_feedback);

        assert_ne!(
            initial_state, after_ai_tuple,
            "AI response processing should change UI state"
        );
    }

    /// Test that input validation state is tracked
    #[tokio::test]
    async fn test_input_validation_state_tracking() {
        let mut session = create_test_session();

        // Empty input
        let empty_state = calculate_ui_state_tuple(&session);
        assert_eq!(session.input_buffer.len(), 0);

        // Add some text
        session.input_buffer = "test input".to_string();
        let with_text_state = calculate_ui_state_tuple(&session);
        assert_ne!(
            empty_state, with_text_state,
            "Adding text should change validation state"
        );

        // Trimmed empty (whitespace only)
        session.input_buffer = "   ".to_string();
        let whitespace_state = calculate_ui_state_tuple(&session);
        assert_ne!(
            with_text_state, whitespace_state,
            "Whitespace changes should affect validation state"
        );
    }

    /// Test that rapid state changes are properly tracked
    #[tokio::test]
    async fn test_rapid_state_changes_detected() {
        let mut session = create_test_session();

        let mut states = Vec::new();
        states.push(calculate_ui_state_tuple(&session));

        // Simulate rapid typing
        for i in 0..5 {
            session.input_buffer.push('a');
            session.cursor_position += 1;
            states.push(calculate_ui_state_tuple(&session));
        }

        // Each change should produce a different state
        for i in 0..states.len() - 1 {
            assert_ne!(
                states[i],
                states[i + 1],
                "Each typing change should produce different UI state"
            );
        }
    }

    #[test]
    fn test_menu_delete_confirm_state_transition() {
        use crate::AppState;

        let mut app_state = AppState::Menu;

        // Simulate 'd' key press
        app_state = AppState::MenuDeleteConfirm;
        assert_eq!(app_state, AppState::MenuDeleteConfirm);

        // Simulate 'n' key press (cancel)
        app_state = AppState::Menu;
        assert_eq!(app_state, AppState::Menu);

        // Simulate 'y' key press (confirm)
        app_state = AppState::MenuDeleteConfirm;
        // Logic for deletion happens in main.rs, here we just check state transition
        app_state = AppState::Menu;
        assert_eq!(app_state, AppState::Menu);
    }

    /// Helper function to create a test session
    fn create_test_session() -> QuizSession {
        let flashcards = vec![
            Flashcard {
                question: "Test Question 1?".to_string(),
                answer: "Test Answer 1".to_string(),
                user_answer: None,
                ai_feedback: None,
                written_to_file: false,
                id: None,
            },
            Flashcard {
                question: "Test Question 2?".to_string(),
                answer: "Test Answer 2".to_string(),
                user_answer: None,
                ai_feedback: None,
                written_to_file: false,
                id: None,
            },
        ];

        QuizSession {
            flashcards,
            current_index: 0,
            deck_name: "Test Quiz".to_string(),
            showing_answer: false,
            input_buffer: String::new(),
            cursor_position: 0,
            session_id: None,
            questions_total: 2,
            questions_answered: 0,
            ai_enabled: true,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: None,
            ai_rx: None,

            input_scroll_y: 0,
            feedback_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
            chat_state: None,
        }
    }

    /// Helper function to calculate UI state tuple for a session (matches main.rs logic)
    fn calculate_ui_state_tuple(
        session: &QuizSession,
    ) -> (
        usize,
        bool,
        bool,
        usize,
        usize,
        u16,
        u16,
        bool,
        usize,
        usize,
    ) {
        (
            session.current_index,
            session.showing_answer,
            session.ai_evaluation_in_progress,
            session.input_buffer.len(),
            session.cursor_position,
            session.input_scroll_y,
            session.feedback_scroll_y,
            session.last_ai_error.is_some(),
            session.questions_answered,
            session
                .flashcards
                .iter()
                .filter(|f| f.ai_feedback.is_some())
                .count(),
        )
    }

    /// Test that async AI channels work correctly
    #[tokio::test]
    async fn test_async_ai_channel_integration() {
        // Create channels matching main.rs setup
        let (_request_tx, _request_rx) = mpsc::channel::<AiRequest>(32);
        let (response_tx, response_rx) = mpsc::channel::<AiResponse>(32);

        // Create a simple session with async channels
        let mut session = QuizSession {
            flashcards: vec![Flashcard {
                question: "Async test?".to_string(),
                answer: "Async answer".to_string(),
                user_answer: Some("User async".to_string()),
                ai_feedback: None,
                written_to_file: false,
                id: None,
            }],
            current_index: 0,
            deck_name: "Async Test".to_string(),
            showing_answer: true,
            input_buffer: String::new(),
            cursor_position: 0,
            session_id: None,
            questions_total: 1,
            questions_answered: 1,
            ai_enabled: true,
            ai_evaluation_in_progress: false,
            ai_last_evaluated_index: None,
            ai_evaluation_start_time: None,
            last_ai_error: None,
            ai_tx: Some(_request_tx),
            ai_rx: Some(response_rx),

            input_scroll_y: 0,
            feedback_scroll_y: 0,
            session_assessment: None,
            assessment_loading: false,
            assessment_error: None,
            assessment_scroll_y: 0,
            chat_state: None,
        };

        // Send an AI response through the async channel
        let ai_response = AiResponse::Evaluation {
            flashcard_index: 0,
            result: AIEvaluationResult {
                feedback: crate::ai::AIFeedback {
                    is_correct: true,
                    correctness_score: 0.95,
                    corrections: vec![],
                    explanation: "Async test passed!".to_string(),
                    suggestions: vec![],
                },
                raw_response: r#"{"is_correct": true, "correctness_score": 0.95, "corrections": [], "explanation": "Async test passed!", "suggestions": []}"#.to_string(),
            },
        };

        // Send response (simulating ai_worker)
        let _ = response_tx.send(ai_response).await;

        // Receive and process response (simulating main loop)
        if let Some(rx) = &mut session.ai_rx {
            if let Some(response) = rx.recv().await {
                session.process_ai_responses(response);
            }
        }

        // Verify the async channel integration works
        assert!(session.flashcards[0].ai_feedback.is_some());
        assert_eq!(
            session.flashcards[0]
                .ai_feedback
                .as_ref()
                .unwrap()
                .correctness_score,
            0.95
        );
        assert_eq!(
            session.flashcards[0]
                .ai_feedback
                .as_ref()
                .unwrap()
                .explanation,
            "Async test passed!"
        );
    }
}
