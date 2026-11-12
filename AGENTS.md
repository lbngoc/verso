# Agent notes

## Commits

- Group at the size of **one user story or one major phase**: a handful of substantial commits, each one
  coherent unit of work.
- Do **not** split a single US/phase into micro-commits (per file, per step, per small fix) unless the parts
  are genuinely independent or the user asks for finer commits.
- Subject: imperative, conventional prefix (`feat:`, `fix:`, `docs:`, `build:`, `chore:`, `test:`); the body
  explains why, not what the diff already shows.
- Run the checks before committing; never commit build output, logs, secrets or generated artifacts.

## Repository

- The planning workspace (`specs/`, `.specify/`, `.claude/`) is local and ignored; it is not published.
- User-facing documents live in `docs/public/` and are synced to `lbngoc/verso-translate` by the release
  pipeline.
