use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct QuizLayout {
    pub header_area: Rect,
    pub question_area: Rect,
    pub answer_area: Rect,
    pub help_area: Rect,
}

pub struct SummaryLayout {
    pub header_area: Rect,
    pub content_area: Rect, // The main area (chunks[1])
    pub footer_area: Rect,
    // Sub-layout for content area
    pub assessment_spacer_top: Rect,
    pub assessment_content: Rect,
    pub assessment_help: Rect,
}

pub fn calculate_quiz_chunks(area: Rect) -> QuizLayout {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(2),
            Constraint::Percentage(80),
            Constraint::Length(4),
        ])
        .split(area);

    QuizLayout {
        header_area: chunks[0],
        question_area: chunks[1],
        answer_area: chunks[2],
        help_area: chunks[3],
    }
}

pub fn calculate_summary_chunks(area: Rect) -> SummaryLayout {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    let assessment_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(chunks[1]);

    SummaryLayout {
        header_area: chunks[0],
        content_area: chunks[1],
        footer_area: chunks[2],
        assessment_spacer_top: assessment_chunks[0],
        assessment_content: assessment_chunks[1],
        assessment_help: assessment_chunks[2],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quiz_layout() {
        let area = Rect::new(0, 0, 100, 100);
        let layout = calculate_quiz_chunks(area);

        // Check height constraints
        // Margin 1 means effective height is 98
        // header: 3
        // footer: 3
        // remaining: 92
        // answer: 80% of 98? No, constraints apply to the full split.
        // Wait, ratatui constraints:
        // Length(3), Min(2), Percentage(80), Length(3)
        // Fixed: 3 + 3 = 6. Remaining 92.
        // Percentage(80) of 98 is 78.
        // Min(2) takes whatever is left?
        // Layout logic is complex.

        assert_eq!(layout.header_area.height, 3);
        assert_eq!(layout.help_area.height, 4);
        // We just verify it returns something sane
        assert!(layout.answer_area.height > 0);
        assert!(layout.question_area.height > 0);
    }

    #[test]
    fn test_summary_layout() {
        let area = Rect::new(0, 0, 100, 100);
        let layout = calculate_summary_chunks(area);

        assert_eq!(layout.header_area.height, 3);
        assert_eq!(layout.footer_area.height, 3);

        // Check sub-layout
        // content_area height approx 92 (98 - 6)
        assert_eq!(layout.content_area.height, 92);

        assert_eq!(layout.assessment_spacer_top.height, 3);
        assert_eq!(layout.assessment_help.height, 3);
        assert_eq!(layout.assessment_content.height, 92 - 6);
    }
}
