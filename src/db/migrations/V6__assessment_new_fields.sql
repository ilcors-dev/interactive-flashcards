-- Add new assessment fields for more actionable feedback
ALTER TABLE session_assessments ADD COLUMN key_concepts_to_review TEXT NOT NULL DEFAULT '[]';
ALTER TABLE session_assessments ADD COLUMN misconceptions TEXT NOT NULL DEFAULT '[]';
ALTER TABLE session_assessments ADD COLUMN priority_questions TEXT NOT NULL DEFAULT '[]';
ALTER TABLE session_assessments ADD COLUMN study_focus TEXT NOT NULL DEFAULT '';
