CREATE TABLE IF NOT EXISTS session_assessments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL UNIQUE,
    grade_percentage REAL NOT NULL,
    mastery_level TEXT NOT NULL,
    overall_feedback TEXT NOT NULL,
    suggestions TEXT NOT NULL,
    strengths TEXT NOT NULL,
    weaknesses TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

CREATE INDEX IF NOT EXISTS idx_assessments_session ON session_assessments(session_id);
CREATE INDEX IF NOT EXISTS idx_assessments_grade ON session_assessments(grade_percentage);
