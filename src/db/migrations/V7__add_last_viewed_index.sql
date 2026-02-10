-- Track the last viewed question index for session resume
ALTER TABLE sessions ADD COLUMN last_viewed_index INTEGER NOT NULL DEFAULT 0;
