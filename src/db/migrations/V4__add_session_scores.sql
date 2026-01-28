-- V4__add_session_scores.sql
ALTER TABLE sessions ADD COLUMN current_score REAL DEFAULT 0.0;
