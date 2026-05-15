# AGENTS.md

## Project Mission

`content-score` is a local Rust CLI for turning content judgment into a repeatable loop:

```text
candidate-score -> score -> predict -> retro -> calibrate -> upgrade
```

The target use case is Douyin/self-media content creation, but the core method is platform-neutral: score ideas and scripts before publishing, record a falsifiable pre-publication bet, enter real performance after publishing, then calibrate the rubric against actual outcomes.

This project is not a Codex/Claude skill by itself. The CLI is the durable local system of record. The Codex skill at `~/.codex/skills/content-score` is only the workflow bridge that routes natural-language requests into this CLI.

## Code Forces We Are Building On

1. **Our Rust CLI (`/home/tt/content-score`)**
   - Owns durable state, validation, scoring math, prediction hashes, retros, calibration, and rubric upgrade proposals.
   - This is the source of truth for persisted scores and performance data.

2. **`cheat-on-content` design lessons (`/home/tt/cheat-on-content`)**
   - We borrowed the core loop: score -> blind prediction -> retro -> rubric evolution.
   - We borrowed the seven-dimension starter rubric: `ER`, `HP`, `QL`, `NA`, `AB`, `SR`, `SAT`.
   - We borrowed the discipline that predictions must be written before seeing real performance data.
   - We are not directly cloning its skill ecosystem; we are implementing a smaller, local, product-like CLI.
   - Attribution: thank `XBuilderLAB/cheat-on-content` and its creator in public project docs.

3. **Codex skill bridge (`~/.codex/skills/content-score`)**
   - Turns user requests like "score this Douyin script" or "data is back, do retro" into `content-score` CLI calls.
   - The skill must not replace the CLI with chat-only analysis.
   - The repository copy lives at `skills/content-score`; keep it in sync with the installed local skill.

4. **Douyin semi-automatic adapter**
   - `src/douyin.rs` owns the Rust CLI command surface and delegates live browser work to a Python adapter.
   - `adapters/douyin-session/cli.py` provides `doctor`, `login`, and `fetch` using Playwright.
   - `adapters/douyin-session/normalize.py` normalizes fetched Douyin responses into the same JSON shape used by `retro import`.
   - Live fetches require user-authorized Douyin login state and may need maintenance when Douyin changes Creator Center or public-page behavior.
   - `TrendRadar` is a reference direction for trend-source integration, not a current dependency.

## Current Architecture

Main source files:

- `src/main.rs`: CLI parsing and command dispatch.
- `src/dimensions.rs`: seven dimensions and code parsing.
- `src/rubric.rs`: active rubric and composite formula.
- `src/score.rs`: score parsing, strict JSON score ingestion, optional OpenAI-compatible LLM path.
- `src/storage.rs`: local `.content-score/content.sqlite` schema and persistence.
- `src/prediction.rs`: prediction markdown rendering and hash integrity.
- `src/retro_import.rs`: CSV/JSON batch retro import parsing and row-level reporting.
- `src/douyin.rs`: `content-score douyin doctor/login/fetch`, adapter invocation, input validation, JSON backup, auto-import, and duplicate/replace safeguards.
- `src/calibration.rs`: completed-sample analysis and conservative weight proposal logic.
- `src/upgrade.rs`: rubric version increment logic.
- `adapters/douyin-session/cli.py`: Playwright-based Douyin adapter entrypoint.
- `adapters/douyin-session/normalize.py`: adapter response normalization and import-row construction.
- `adapters/douyin-session/tests/test_normalize.py`: Python normalization tests.
- `tests/cli_smoke.rs`: end-to-end CLI smoke tests.
- `skills/content-score/SKILL.md`: distributable Codex skill for invoking the CLI.
- `skills/content-score/agents/openai.yaml`: UI metadata for the packaged skill.
- `skills/content-score/references/scoring.md`: score JSON and dimension guidance for the skill.

Local user project state:

```text
.content-score/
  content.sqlite
  rubric.toml
  imports/
  douyin-debug/
.auth/
predictions/
```

Installed CLI path:

```bash
/home/tt/.cargo/bin/content-score
```

Fallback command when the installed binary is missing:

```bash
cargo run --manifest-path /home/tt/content-score/Cargo.toml -- <args>
```

## Completed Goals

### Core CLI

- Rust project scaffolded and committed.
- User-local Rust toolchain installed under `~/.cargo`.
- CLI installed to `/home/tt/.cargo/bin/content-score`.

### Scoring

- Implemented seven starter dimensions:
  - `ER`: Emotional Resonance
  - `HP`: Hook Potential
  - `QL`: Quotable Lines
  - `NA`: Narrativity
  - `AB`: Audience Breadth
  - `SR`: Social Resonance
  - `SAT`: Satire Depth
- Implemented v0 composite formula:

```text
composite = (ER + HP + QL + NA + AB + SR + SAT) / 7 * 2.0
```

- Scores are validated as integer `0..=5`.
- All seven dimensions are required.
- Supports manual `--scores`.
- Supports strict `--score-json`.
- Supports optional OpenAI-compatible `--llm` path.

### Candidate Scoring

- `content-score candidates add <text>`
- `content-score candidates score <id> ...`
- `content-score candidates top`
- Candidate scores are treated as prioritization signals, not predictions.

### Script Scoring

- `content-score score <script.md> ...`
- Stores score runs in SQLite.
- Outputs dimension table and composite.

### Prediction Loop

- `content-score predict <script.md> ... --bet <text>`
- Writes human-readable markdown under `predictions/`.
- Stores prediction hash in SQLite.
- Records script hash, rubric version, scores, composite, bet, and optional bucket.

### Retro Loop

- `content-score retro <prediction-id> --plays ... --likes ... --comments ... --shares ... --saves ...`
- Checks prediction markdown hash before recording retro.
- Marks retro contaminated if the prediction file changed.
- `content-score retro import <file.csv|file.json>`
- Imports multiple retros in one run.
- Continues after row-level failures and reports imported/failed/contaminated counts.

### Duplicate Retro Safeguards

- `retro import` rejects duplicate completed samples for a prediction by default.
- Import callers can opt into replace behavior where supported.
- Douyin fetch rejects a second import for the same prediction unless `--replace` is supplied.

### Calibration And Upgrade

- `content-score calibrate`
- Analyzes completed, uncontaminated samples.
- Compares average plays for high-score (`>=4`) vs low-score dimensions.
- `content-score upgrade --propose`
- `content-score upgrade --apply <id>`
- Upgrades are explicit and confirmed; no silent auto-upgrade.

### Codex Skill Integration

- Added distributable skill files under `skills/content-score/`.
- Created `~/.codex/skills/content-score/SKILL.md`.
- Added `~/.codex/skills/content-score/references/scoring.md`.
- Validated skill structure with `quick_validate.py`.
- Skill now follows the stronger `imagegen` style:
  - top-level modes
  - hard rules
  - when to use / when not to use
  - decision tree
  - score JSON policy
  - CLI fallback
  - common mistakes

### Douyin Semi-Automatic Adapter

- `content-score douyin doctor`
- `content-score douyin login`
- `content-score douyin fetch <prediction-id> <url-or-id>`
- Fetch supports raw aweme ids, long Douyin video URLs, and `v.douyin.com` short links.
- Fetch writes `.content-score/imports/douyin-<prediction-id>.json`.
- Fetch imports by default through the same retro import path.
- `--no-import` and `--dry-run` keep the JSON backup without recording a retro.
- `--replace` replaces an existing retro for the prediction.
- `doctor` reports project and ignore-file status before delegating dependency checks to the adapter.
- `login` and `fetch` run inside an initialized content project; Rust passes the current project root to the adapter as `CONTENT_SCORE_PROJECT_ROOT`.
- Auth/session data stays in the user's project `.auth/`; debug files stay under `.content-score/douyin-debug/`.

### Verification

Most recent full verification passed:

```bash
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings
python3 -m unittest discover adapters/douyin-session/tests -v
```

Current automated suite shape:

- 13 Rust unit tests.
- 28 Rust CLI smoke tests.
- 11 Python adapter tests under `adapters/douyin-session/tests`.
- Live Douyin login/fetch has not been verified in this environment because Playwright/Chromium/login state is not installed here.

## Prepared Goals

### Completed Target: Batch Retro Import

Batch retro import reduces manual retro entry and remains the stable ingestion path used by the Douyin adapter.

Commands:

```bash
content-score retro import douyin.csv
content-score retro import douyin.json
```

CSV shape:

```csv
prediction_id,plays,likes,comments,shares,saves,top_comments,notes
2026-05-15_xxx,1200,80,12,4,9,"comment1|comment2|comment3","T+3"
```

JSON shape:

```json
[
  {
    "prediction_id": "2026-05-15_xxx",
    "plays": 1200,
    "likes": 80,
    "comments": 12,
    "shares": 4,
    "saves": 9,
    "top_comments": ["comment1", "comment2", "comment3"],
    "notes": "T+3"
  }
]
```

Implemented behavior:

- Import multiple retros in one run.
- Reuse the same prediction hash integrity check as single `retro`.
- Do not abort the whole import because one row fails.
- Print success/failure/contaminated counts.
- Report row-level errors clearly.
- Store `top_comments` in a stable textual format.
- CLI smoke tests cover CSV and JSON import.

### Completed Target: Duplicate/Validation Safeguards

- Existing retro rows are detected before duplicate insertion.
- Default import behavior rejects duplicates.
- Replace behavior is explicit where implemented.
- Douyin fetch refuses duplicate default imports before live adapter execution.

### Completed Target: Douyin Semi-Automatic Adapter

- Rust CLI delegates `doctor`, `login`, and `fetch` to the Python adapter.
- Auth/session data is local to the user's content project.
- Fetch converts one Douyin record into the standard retro import JSON path.
- Auto-import is default, with `--no-import`, `--dry-run`, and `--replace` controls.
- Unit and smoke tests cover command routing, URL/id validation, backup JSON, auto-import, and duplicate behavior.
- Live Douyin login/fetch remains manual verification because it depends on Playwright, Chromium, current Douyin behavior, and an authorized user session.

### Next Target: Better Operational UX

- Add `list` / status commands for predictions, completed retros, and pending retros.
- Improve operator guidance when Douyin adapter prerequisites or login state are missing.
- Keep the installed Codex skill synchronized when CLI behavior changes.

### Later Target: Better Calibration

The current calibration is intentionally simple. Future improvements:

- Require minimum sample counts before recommending weight changes.
- Add rank correlation between composite and actual plays.
- Add percentile/bucket calibration once enough samples exist.
- Distinguish low confidence from negative signal.
- Add versioned calibration reports.

### Later Target: Rubric Evolution Beyond Weights

First version adjusts weights only. Later:

- Support dimension definition notes.
- Support adding/removing dimensions only after enough samples.
- Support old-version rescore comparison.
- Keep old rubric versions auditable.

## Evaluation

### What Is Strong

- The project already has the essential closed loop.
- Persistence is local and simple.
- Prediction hash integrity exists from the beginning.
- Batch retro import now lowers manual data-entry friction.
- Upgrades are confirm-first, reducing accidental overfitting.
- Skill integration exists but does not replace the CLI.
- The architecture is small enough to reason about and test.

### What Is Still Weak

- Retro data entry is still manual at the data-source level, but can now be batched.
- Douyin semi-automatic fetch exists, but live browser behavior is not continuously verified.
- Calibration is statistically crude with small samples.
- LLM scoring is only an API-compatible path; there is no robust provider configuration UX.
- Prediction markdown locking is hash-based, not filesystem-enforced.
- No `list` command despite the original design mentioning it.

### Biggest Current Gap

The biggest practical gap is not scoring. It is data recovery:

```text
published Douyin performance -> clean retro records -> calibration pool
```

Batch import and Douyin fetch make ingestion cheaper, but the system still depends on clean source data and user-authorized Douyin browser sessions.

### Recommended Priority Order

1. Add `list` / status commands for predictions and pending retros.
2. Improve Douyin adapter prerequisite/login diagnostics.
3. Update the installed Codex skill whenever CLI retro-import or Douyin behavior changes.
4. Add stronger calibration math.
5. Consider trend-source integrations such as TrendRadar.

## Development Rules For Future Agents

- Use TDD for behavior changes.
- Keep CLI state changes durable in SQLite.
- Do not introduce chat-only scoring paths that skip CLI persistence.
- Do not silently mutate active rubric weights.
- Do not bypass prediction hash checks.
- Do not store Douyin auth/session data in this repository.
- Use `rg` for code search.
- Use `apply_patch` for manual edits.
- After changes, run:

```bash
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings
```

## Current Status

The project is ready for the next implementation task: **better operational UX for prediction status and Douyin adapter diagnostics**.
