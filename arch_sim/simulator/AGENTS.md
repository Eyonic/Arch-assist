# Repository Guidelines

## Project Structure & Module Organization
- Root entrypoint `main.py` starts a simple REPL; `ai` prefixed input is rewritten by the AI pipeline before execution.
- Core simulator code lives in `simulator/`: `runner.py` dispatches commands, `pacman.py` / `paru.py` / `systemctl.py` / `net.py` emulate subsystems, `state.py` holds mutable system data, and `scenarios.py` mutates the state for preset failure cases.
- AI glue: `ai_runner.py` orchestrates translation and safety checks; `ai_openai.py` calls OpenAI with a strict system prompt; `safety.py` blocks disallowed commands.
- Tests belong in `tests/`; `pytest.ini` already sets the test root. Keep caches (`.pytest_cache/`, `__pycache__/`) ignored.

## Build, Test, and Development Commands
- Create an environment and install deps: `python -m venv .venv && .venv\\Scripts\\activate && pip install openai python-dotenv pytest`.
- Run the simulator REPL from the repo root: `python main.py`.
- Execute tests: `pytest` (add `-q` for terse output). Prefer mocking OpenAI in unit tests to avoid live calls.

## Coding Style & Naming Conventions
- Python 3, 4-space indentation, UTF-8/ASCII-only unless required. Modules stay lowercase with underscores; classes use PascalCase; functions/variables use snake_case.
- Keep simulator handlers pure except for explicit `SystemState` mutations. Mirror existing patterns (split the command, guard rails, return deterministic strings).
- When extending safety, update both `ALLOWED_PREFIXES` and `FORBIDDEN` consistently and add coverage.

## Testing Guidelines
- Use `pytest` with `test_*.py` naming. Favor small, state-driven tests that instantiate `SystemState`, `Simulator`, and (optionally) `AIRunner`.
- Stub `ai_openai.translate` when possible to avoid network usage; assert both the rewritten command and the simulator output.
- Add regression tests whenever modifying command parsing, error flags, or scenarios to prevent drift in user-facing text.

## Commit & Pull Request Guidelines
- Commit messages in this repo are short and descriptive (e.g., `arch linux simulator`, `update readme`); keep using imperative, lowercase phrases without trailing punctuation.
- In PRs, include: clear summary of behavior change, mention affected modules (e.g., `runner.py`, `safety.py`), steps to verify (`pytest`, manual REPL run), and linked issues if any. Provide screenshots only when output formatting changes.

## Security & Configuration Tips
- Place `OPENAI_API_KEY=sk-...` in a local `.env`; never commit secrets. The key is validated on import, so missing or malformed keys will raise early.
- Do not relax safety rules without justification. Avoid adding shell features (pipes, redirects, or arbitrary binaries) that bypass the simulator’s guardrails.
