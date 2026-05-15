# Douyin Adapter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `content-score douyin doctor/login/fetch` so a manually supplied prediction and Douyin video can produce a standard retro JSON and, by default, import it.

**Architecture:** Keep browser automation outside the Rust core. Rust validates project state, duplicate policy, command options, adapter invocation, JSON handoff, and import behavior. Python owns Playwright/Chromium login, fetch, response capture, metric normalization, and JSON output.

**Tech Stack:** Rust 2021, `clap`, `rusqlite`, `serde_json`, existing `retro_import`; Python 3, Playwright, fixture-based Python tests for normalization.

---

## File Structure

- Modify `src/storage.rs`: add prediction existence, retro existence, and delete helpers.
- Modify `src/retro_import.rs`: add import options for duplicate handling and replace behavior.
- Create `src/douyin.rs`: URL/ID resolution, adapter path/python selection, doctor/login/fetch orchestration.
- Modify `src/main.rs`: add `mod douyin;` and `douyin` subcommands.
- Modify `tests/cli_smoke.rs`: add CLI smoke tests using a fake adapter, no live Douyin required.
- Create `adapters/douyin-session/requirements.txt`: Playwright dependency.
- Create `adapters/douyin-session/cli.py`: adapter CLI entrypoint.
- Create `adapters/douyin-session/normalize.py`: fixture-testable normalization helpers.
- Create `adapters/douyin-session/tests/test_normalize.py`: Python unit tests for normalization and URL parsing.
- Modify `README.md`, `AGENTS.md`, and `skills/content-score/SKILL.md`: document Douyin commands and boundaries.

---

### Task 1: Duplicate Retro Guards

**Files:**
- Modify: `src/storage.rs`
- Modify: `src/retro_import.rs`
- Test: `tests/cli_smoke.rs`

- [ ] **Step 1: Write failing CLI tests**

Add a test that imports the same `prediction_id` twice and expects the second import to fail by default. Add small test helpers `init_project`, `run_ok`, and `write_retro_json` if they do not already exist.

```rust
#[test]
fn retro_import_rejects_duplicate_prediction_by_default() {
    let temp = tempdir().unwrap();
    init_project(temp.path());
    let prediction_id = create_prediction(temp.path(), "dup.md", "强情绪开头。");
    write_retro_json(temp.path(), "first.json", &prediction_id, 1200);
    write_retro_json(temp.path(), "second.json", &prediction_id, 1800);

    run_ok(temp.path(), ["retro", "import", "first.json"]);

    Command::cargo_bin("content-score")
        .unwrap()
        .current_dir(temp.path())
        .args(["retro", "import", "second.json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("imported: 0"))
        .stdout(predicate::str::contains("failed: 1"))
        .stdout(predicate::str::contains("already has a retro"));
}
```

- [ ] **Step 2: Run red test**

Run:

```bash
cargo test --test cli_smoke retro_import_rejects_duplicate_prediction_by_default
```

Expected: fail because duplicate imports are currently allowed.

- [ ] **Step 3: Implement storage helpers**

Add:

```rust
pub fn prediction_exists(conn: &Connection, id: &str) -> Result<bool>;
pub fn retro_exists(conn: &Connection, prediction_id: &str) -> Result<bool>;
pub fn delete_retros_for_prediction(conn: &Connection, prediction_id: &str) -> Result<usize>;
```

- [ ] **Step 4: Add `ImportOptions`**

In `src/retro_import.rs`, add:

```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct ImportOptions {
    pub replace_existing: bool,
}

pub fn import_file_with_options(
    root: &Path,
    conn: &Connection,
    path: &Path,
    options: ImportOptions,
) -> Result<ImportSummary>;
```

Keep `import_file(...)` as a wrapper using default options.

- [ ] **Step 5: Enforce duplicate policy**

Before `insert_retro`, if a retro exists:

- default: row failure `"prediction already has a retro: <id>"`
- replace: delete old rows, then insert

- [ ] **Step 6: Run focused tests and commit**

Run:

```bash
cargo test --test cli_smoke retro_import_rejects_duplicate_prediction_by_default
cargo test --test cli_smoke retro_import_json_records_rows
```

Commit:

```bash
git add src/storage.rs src/retro_import.rs tests/cli_smoke.rs
git commit -m "feat: guard duplicate retro imports"
```

---

### Task 2: Douyin Command Shape And URL Resolution

**Files:**
- Create: `src/douyin.rs`
- Modify: `src/main.rs`
- Test: `src/douyin.rs`
- Test: `tests/cli_smoke.rs`

- [ ] **Step 1: Write failing tests**

Add unit tests for local URL/ID parsing and a CLI smoke test that the subcommand exists. These require no network and no adapter.

```rust
#[test]
fn resolves_raw_and_long_douyin_inputs() {
    assert_eq!(
        resolve_aweme_id("7333333333333333333").unwrap(),
        "7333333333333333333"
    );
    assert_eq!(
        resolve_aweme_id("https://www.douyin.com/video/7333333333333333333").unwrap(),
        "7333333333333333333"
    );
}
```

- [ ] **Step 2: Run red test**

Run:

```bash
cargo test resolves_raw_and_long_douyin_inputs
```

Expected: fail because `douyin` command does not exist.

- [ ] **Step 3: Add CLI enum**

Add to `src/main.rs`:

```rust
Douyin {
    #[command(subcommand)]
    command: douyin::DouyinCommand,
},
```

- [ ] **Step 4: Implement URL resolution**

In `src/douyin.rs`, implement:

```rust
pub fn resolve_aweme_id(input: &str) -> Result<String>;
```

Rules:

- all digits -> raw ID
- `/video/<digits>` -> extracted ID
- `v.douyin.com` -> accepted as unresolved short link for adapter
- other input -> error

- [ ] **Step 5: Wire command stubs**

Add `doctor`, `login`, and `fetch` subcommands that parse and print clear stub errors where adapter execution is not yet implemented. Add a smoke assertion that `content-score douyin --help` contains `doctor`, `login`, and `fetch`.

- [ ] **Step 6: Run focused tests and commit**

Run:

```bash
cargo test resolves_raw_and_long_douyin_inputs
cargo test --test cli_smoke douyin_help_lists_subcommands
```

Commit:

```bash
git add src/main.rs src/douyin.rs tests/cli_smoke.rs
git commit -m "feat: add douyin command shape"
```

---

### Task 3: Fake Adapter Handoff And Auto Import

**Files:**
- Modify: `src/douyin.rs`
- Modify: `tests/cli_smoke.rs`

- [ ] **Step 1: Write fake adapter helper**

In `tests/cli_smoke.rs`, add a fake adapter script writer. The script should accept `fetch <input> --prediction-id <id> --output <path>` and write the standard JSON array.

```rust
fn write_fake_douyin_adapter(root: &std::path::Path, plays: i64) -> PathBuf {
    let path = root.join("fake-douyin-adapter.py");
    fs::write(
        &path,
        format!(
            r#"#!/usr/bin/env python3
import json, sys
out = sys.argv[sys.argv.index("--output") + 1]
prediction_id = sys.argv[sys.argv.index("--prediction-id") + 1]
json.dump([{{"prediction_id": prediction_id, "plays": {plays}, "likes": 80, "comments": 12, "shares": 4, "saves": 9, "top_comments": ["评论1"], "notes": "fake douyin"}}], open(out, "w", encoding="utf-8"), ensure_ascii=False)
print("aweme_id: 7333333333333333333")
"#
        ),
    )
    .unwrap();
    path
}
```

- [ ] **Step 2: Write tests for default import and `--no-import`**

Assert:

- default fetch creates JSON and calibration sees one sample
- `--no-import` creates JSON but calibration still sees zero samples

- [ ] **Step 3: Run red tests**

Run:

```bash
cargo test --test cli_smoke douyin_
```

Expected: fail until adapter invocation and import handoff exist.

- [ ] **Step 4: Implement adapter invocation**

In `src/douyin.rs`:

- choose adapter path from `CONTENT_SCORE_DOUYIN_ADAPTER`, otherwise repository adapter path
- choose Python from project `.venv/bin/python`, otherwise `python3`
- write JSON to `.content-score/imports/douyin-<prediction-id>.json`
- pass `--prediction-id` and `--output`

- [ ] **Step 5: Implement default import / `--no-import`**

After successful adapter run:

- default: call `retro_import::import_file_with_options`
- `--no-import`: skip import and print `imported: no`

- [ ] **Step 6: Run focused tests and commit**

Run:

```bash
cargo test --test cli_smoke douyin_
```

Commit:

```bash
git add src/douyin.rs tests/cli_smoke.rs
git commit -m "feat: import douyin adapter output"
```

---

### Task 4: Dry Run, Replace, Doctor, And Login Orchestration

**Files:**
- Modify: `src/douyin.rs`
- Modify: `tests/cli_smoke.rs`

- [ ] **Step 1: Add failing tests**

Cover:

- `--dry-run` writes/fetches but does not import
- duplicate default fetch fails before adapter execution
- `--replace` replaces old retro
- `--replace --no-import` fails as invalid
- `douyin doctor` delegates to fake adapter
- `douyin login` delegates to fake adapter

- [ ] **Step 2: Run red tests**

Run:

```bash
cargo test --test cli_smoke douyin_
```

Expected: fail for unimplemented flags/commands.

- [ ] **Step 3: Implement preflight**

Before launching adapter for default import:

- verify prediction exists
- if retro exists and no `--replace`, fail before adapter execution
- allow `--no-import` even when retro exists

- [ ] **Step 4: Implement replace**

For `--replace`:

- invoke adapter
- delete existing retro rows
- import new JSON
- print `replaced: yes`

- [ ] **Step 5: Implement doctor/login delegation**

`doctor` and `login` call the adapter with the same subcommand and stream output. `doctor` should not require `.auth/`.

- [ ] **Step 6: Run focused tests and commit**

Run:

```bash
cargo test --test cli_smoke douyin_
```

Commit:

```bash
git add src/douyin.rs tests/cli_smoke.rs
git commit -m "feat: finish douyin cli orchestration"
```

---

### Task 5: Python Adapter And Fixture Tests

**Files:**
- Create: `adapters/douyin-session/requirements.txt`
- Create: `adapters/douyin-session/cli.py`
- Create: `adapters/douyin-session/normalize.py`
- Create: `adapters/douyin-session/tests/test_normalize.py`

- [ ] **Step 1: Write Python fixture tests**

Use `unittest` so no extra test dependency is required. Cover:

- `parse_aweme_input` raw ID
- `parse_aweme_input` long URL
- `normalize_video` maps play/like/comment/share/save fields
- `normalize_comments` sorts top comments by like count
- missing required metrics raises a clear error

- [ ] **Step 2: Run red tests**

Run:

```bash
python3 -m unittest discover adapters/douyin-session/tests -v
```

Expected: fail because files do not exist yet.

- [ ] **Step 3: Implement normalization helpers**

In `normalize.py`, implement pure functions:

```python
def parse_aweme_input(raw: str) -> str: ...
def normalize_video(raw: dict) -> dict: ...
def normalize_comments(raw_comments: list[dict], limit: int = 20) -> list[str]: ...
def build_import_row(prediction_id: str, video: dict, comments: list[str], notes: str) -> dict: ...
```

- [ ] **Step 4: Implement adapter CLI**

`cli.py` supports:

```bash
python cli.py doctor
python cli.py login
python cli.py fetch <url-or-id> --prediction-id <id> --output <path>
```

First implementation may adapt the known `cheat-on-content/adapters/perf-data/douyin-session` Playwright flow, but must output the standard JSON array and keep auth/debug under the current content project.

- [ ] **Step 5: Run Python tests and commit**

Run:

```bash
python3 -m unittest discover adapters/douyin-session/tests -v
```

Commit:

```bash
git add adapters/douyin-session
git commit -m "feat: add douyin playwright adapter"
```

---

### Task 6: Docs, Skill, And Final Verification

**Files:**
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `skills/content-score/SKILL.md`
- Optionally modify: `.gitignore`

- [ ] **Step 1: Update docs**

Document:

- `content-score douyin doctor`
- `content-score douyin login`
- `content-score douyin fetch <prediction-id> <url-or-id>`
- `--no-import`, `--dry-run`, `--replace`
- dependency setup commands
- `.auth/` and debug-file warnings

- [ ] **Step 2: Update skill**

Remove Douyin scraping from "when not to use" and add a Douyin fetch workflow that still respects prediction discipline and default auto-import.

- [ ] **Step 3: Validate skill and run full verification**

Run:

```bash
python3 /home/tt/.codex/skills/.system/skill-creator/scripts/quick_validate.py skills/content-score
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings
python3 -m unittest discover adapters/douyin-session/tests -v
```

- [ ] **Step 4: Manual live checklist**

In a scratch content project with Playwright installed:

```bash
content-score init
content-score douyin doctor
content-score douyin login
content-score douyin fetch <prediction-id> <real-douyin-url>
content-score calibrate
```

Manual live Douyin may be skipped in CI or non-interactive sessions, but must be reported honestly.

- [ ] **Step 5: Commit and push**

```bash
git add README.md AGENTS.md skills/content-score/SKILL.md .gitignore
git commit -m "docs: document douyin adapter workflow"
git push
```

---

## Self-Review

- Spec coverage: plan covers doctor, login, fetch, default auto-import, JSON backup, `--no-import`, `--dry-run`, `--replace`, duplicate rejection, short/long/raw input support, adapter isolation, docs, and tests.
- Live Douyin access is intentionally isolated to manual verification; automated tests use fake adapters and fixtures.
- The first implementation should not add automatic matching, batch recent-video fetching, hotspot crawling, or direct SQLite writes from Python.
