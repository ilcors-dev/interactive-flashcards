## Interactive Flashcards

Terminal flashcard app. Pick a CSV deck, answer questions, and review a summary. Optional AI evaluation is enabled when `OPENROUTER_API_KEY` is set.

### Why?

I started to use NotebookLM to generate flashcards and review them, but it lacks a way of typing and storing the answers which I personally find useful to remember the things I'm studying. So I made this (entirely by vibe coding because I was studying for an exam in the meantime).

### CSV format

Each line is a pair: `question,answer`. Quotes are supported for commas and quotes.

Example:
```csv
"What is 2+2?","4"
"What does a comma do?","It separates fields"
```

### Storage

- Flashcards: `~/.local/share/interactive-flashcards/flashcards/*.csv` (Windows: `%USERPROFILE%\.local\share\interactive-flashcards\flashcards\*.csv`)
- Database: `~/.local/share/interactive-flashcards/if.db` (Windows: `%USERPROFILE%\.local\share\interactive-flashcards\if.db`)
- Logs: `~/.local/share/interactive-flashcards/ai_debug.log` (Windows: `%USERPROFILE%\.local\share\interactive-flashcards\ai_debug.log`)

### Run

1. Clone the repo and `cd` into it.
2. Build with `cargo build --release`.
3. Run with `cargo run`. To enable AI evaluation, set `OPENROUTER_API_KEY` env var before running or run

```bash
OPENROUTER_API_KEY="..." cargo run
```

### Warning

Most of the application was vibe-coded, it contains SLOP and some bugs.
