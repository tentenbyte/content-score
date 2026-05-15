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

3. **Codex skill bridge (`~/.codex/skills/content-score`)**
   - Turns user requests like "score this Douyin script" or "data is back, do retro" into `content-score` CLI calls.
   - The skill must not replace the CLI with chat-only analysis.

4. **Future Douyin import/crawling work**
   - `cheat-on-content/adapters/perf-data/douyin-session` is the reference for possible Playwright/session-based Douyin Creator Center data capture.
   - First priority is not full browser automation. First priority is batch retro import from CSV/JSON because it is simpler, testable, and immediately useful.

## Current Architecture

Main source files:

- `src/main.rs`: CLI parsing and command dispatch.
- `src/dimensions.rs`: seven dimensions and code parsing.
- `src/rubric.rs`: active rubric and composite formula.
- `src/score.rs`: score parsing, strict JSON score ingestion, optional OpenAI-compatible LLM path.
- `src/storage.rs`: local `.content-score/content.sqlite` schema and persistence.
- `src/prediction.rs`: prediction markdown rendering and hash integrity.
- `src/calibration.rs`: completed-sample analysis and conservative weight proposal logic.
- `src/upgrade.rs`: rubric version increment logic.
- `tests/cli_smoke.rs`: end-to-end CLI smoke tests.

Local user project state:

```text
.content-score/
  content.sqlite
  rubric.toml
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

### Calibration And Upgrade

- `content-score calibrate`
- Analyzes completed, uncontaminated samples.
- Compares average plays for high-score (`>=4`) vs low-score dimensions.
- `content-score upgrade --propose`
- `content-score upgrade --apply <id>`
- Upgrades are explicit and confirmed; no silent auto-upgrade.

### Codex Skill Integration

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

### Verification

Most recent full verification passed:

```bash
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings
```

At that point the test suite included 8 unit tests and 5 CLI smoke tests.

## Prepared Goals

### Next Target: Batch Retro Import

We want to reduce manual retro entry before attempting full Douyin automation.

Proposed command:

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

Required behavior:

- Import multiple retros in one run.
- Reuse the same prediction hash integrity check as single `retro`.
- Do not abort the whole import because one row fails.
- Print success/failure/contaminated counts.
- Report row-level errors clearly.
- Store `top_comments` in a stable textual format.
- Add CLI smoke tests for CSV and JSON import.
- Update README and the Codex skill workflow after implementation.

### Later Target: Douyin Semi-Automatic Adapter

Only after CSV/JSON import works:

- Add a Playwright-based local helper or adapter.
- Store auth/session data under the user's content project, not in the CLI repo.
- Prefer Douyin Creator Center over public pages for reliable metrics.
- Capture plays, likes, comments, shares, saves, and top comments.
- Convert fetched records into the same retro import path instead of bypassing CLI validation.

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
- Upgrades are confirm-first, reducing accidental overfitting.
- Skill integration exists but does not replace the CLI.
- The architecture is small enough to reason about and test.

### What Is Still Weak

- Retro data entry is manual and therefore high-friction.
- Calibration is statistically crude with small samples.
- LLM scoring is only an API-compatible path; there is no robust provider configuration UX.
- Prediction markdown locking is hash-based, not filesystem-enforced.
- No batch import yet.
- No Douyin Creator Center automation yet.
- No robust duplicate-retro prevention beyond current schema behavior.
- No `list` command despite the original design mentioning it.

### Biggest Current Gap

The biggest practical gap is not scoring. It is data recovery:

```text
published Douyin performance -> clean retro records -> calibration pool
```

Until this is easier, the system depends on disciplined manual entry. The next engineering investment should make retro ingestion cheap and reliable.

### Recommended Priority Order

1. Add batch retro CSV/JSON import.
2. Add duplicate/validation safeguards around retros.
3. Update the Codex skill to use retro import.
4. Add `list` / status commands for predictions and pending retros.
5. Add stronger calibration math.
6. Explore Douyin Playwright adapter.

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

The project is ready for the next implementation task: **batch retro import from CSV/JSON**.

