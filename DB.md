# Interactive Flashcards - Database Schema

## Overview

The application uses **SQLite** for persistent storage of quiz sessions, with migrations managed by **Refinery**. The database is located at:

- **macOS/Linux**: `~/.local/share/interactive-flashcards/if.db`
- **Windows**: `%USERPROFILE%\.local\share\interactive-flashcards\if.db`

## Table Relationships

```
sessions (1) ────< (N) flashcards
```

| Table | Primary Key | Foreign Key | Relationship |
|-------|-------------|-------------|--------------|
| `sessions` | `id` | - | Parent: one quiz session |
| `flashcards` | `id` | `session_id` → `sessions.id` | Child: questions in session |

## Schema Details

### sessions Table

Stores metadata about each quiz session.

```sql
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    deck_name TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    questions_total INTEGER NOT NULL,
    questions_answered INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_sessions_deck ON sessions(deck_name);
CREATE INDEX idx_sessions_completed ON sessions(completed_at);
```

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key, auto-incremented |
| `deck_name` | TEXT | Name of the CSV deck file |
| `started_at` | UNIX timestamp | When the session began |
| `completed_at` | UNIX timestamp | NULL until session ends |
| `questions_total` | INTEGER | Total questions in session |
| `questions_answered` | INTEGER | Count of answered questions |
| `created_at` | UNIX timestamp | Row creation time |
| `updated_at` | UNIX timestamp | Last modification time |

### flashcards Table

Stores each question/answer pair within a session.

```sql
CREATE TABLE flashcards (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    question TEXT NOT NULL,
    answer TEXT NOT NULL,
    user_answer TEXT,
    ai_feedback TEXT,
    answered_at INTEGER,
    display_order INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

CREATE INDEX idx_flashcards_session ON flashcards(session_id);
```

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key, auto-incremented |
| `session_id` | INTEGER | Foreign key to sessions.id |
| `question` | TEXT | The flashcard question |
| `answer` | TEXT | The correct answer |
| `user_answer` | TEXT | NULL until user submits |
| `ai_feedback` | TEXT | JSON: AIFeedback object, NULL until AI evaluates |
| `answered_at` | UNIX timestamp | NULL until user submits |
| `display_order` | INTEGER | Preserves shuffled question order |
| `created_at` | UNIX timestamp | Row creation time |
| `updated_at` | UNIX timestamp | Last modification time |

## Data Flow

### Session Lifecycle

```
1. User selects deck
   ↓
2. sessions row created (started_at = now, questions_total = count)
   ↓
3. flashcards rows inserted (one per question, shuffled order)
   ↓
4. User answers question → flashcard.user_answer updated
   ↓
5. sessions.questions_answered incremented
   ↓
6. AI evaluates → flashcard.ai_feedback updated (JSON)
   ↓
7. Repeat steps 4-6 for all questions
   ↓
8. Session complete → sessions.completed_at = now
```

## AIFeedback JSON Schema

AI feedback is stored as JSON in `flashcards.ai_feedback`:

```json
{
  "is_correct": boolean,
  "correctness_score": number (0.0 to 1.0),
  "corrections": string[],
  "explanation": string,
  "suggestions": string[]
}
```

## Migrations

Migrations are managed by **Refinery** and located in `src/db/migrations/`.

### Migration Naming Convention
- Format: `V{version}__{name}.sql`
- Example: `V1__init.sql` (initial schema)

### Applying Migrations
Migrations are embedded at compile time via the `embed_migrations!` macro and run automatically on first database connection.

### Future Migrations
To add a new migration:
1. Create `V2__{feature_name}.sql` in `src/db/migrations/`
2. Include the SQL changes (ALTER TABLE, etc.)
3. Refinery will auto-apply on next run

## Breaking Changes

- **v0.1.0**: TXT file output replaced with SQLite (old TXT sessions not migrated)
- Database is NOT backward compatible with pre-v0.1.0 TXT output files
