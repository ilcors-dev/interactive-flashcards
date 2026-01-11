# Interactive Flashcards

## Overview

The application is designed to help users learn and memorize information through flashcards by engaging in quiz sessions where they answer questions that are read from standard .csv files.
The CSV files contain pairs of questions (first column) and answers (second column).

## Code standards

- Use as little dependecies as possible. Ask before adding new to seek approval.
- Follow the best Rust practices and idioms.
- Each implemented function / functionality must have unit tests attached.
  - Unit tests must cover edge cases.
  - Unit tests must not be flaky and redundant.
- Prior to every change, seek for documentation and / or read git history to understand the context of what is being changed and why.
- Follow consistent code formatting. Use `cargo fmt` to format the code.
- Run `cargo clippy --fix --bin "interactive-flashcards" --allow-dirty` to automatically apply linter suggestions. Fix any remaining warnings manually that cannot be fixed automatically.
- Always keep the PLAN.md and PROGRESS.md updated with the latest changes and description of the application.
  - The PLAN.md must reflect the plans (features) that we intend to implement.
  - The PROGRESS.md must reflect the current state of the implementation.
- When writing code, do not add comments that do not add value. Prefer self-explanatory code over comments that do not add anything useful.
  - Only add comments where the code is complex or non-obvious or part of a choice that needs explanation
