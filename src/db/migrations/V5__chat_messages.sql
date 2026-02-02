CREATE TABLE chat_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    flashcard_id INTEGER NOT NULL,
    session_id INTEGER NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    message_order INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (flashcard_id) REFERENCES flashcards(id),
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

CREATE INDEX idx_chat_messages_flashcard_id ON chat_messages(flashcard_id);
CREATE INDEX idx_chat_messages_session_id ON chat_messages(session_id);
