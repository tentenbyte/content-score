# content-score CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first local Rust CLI for candidate scoring, script scoring, prediction locking, retro entry, calibration, and confirmed rubric upgrades.

**Architecture:** The CLI is a single Rust binary with small modules for dimensions, rubric math, scoring payloads, SQLite storage, prediction integrity, calibration, upgrade proposals, and command handling. Rust owns all deterministic validation and persistence; semantic scoring can come from manual JSON/scores now and an OpenAI-compatible LLM endpoint through environment variables.

**Tech Stack:** Rust 2021, `clap` for CLI parsing, `rusqlite` for local SQLite storage, `serde`/`serde_json`/`toml` for structured data, `sha2` for script and prediction hashes, `reqwest` blocking client for optional LLM scoring.

---

## File Structure

- Create `Cargo.toml`: crate metadata and dependencies.
- Create `src/main.rs`: CLI command definitions and top-level dispatch.
- Create `src/dimensions.rs`: seven dimensions, parsing, validation, and display names.
- Create `src/rubric.rs`: rubric versions, default v0 weights, composite calculation.
- Create `src/score.rs`: score payloads, manual score parsing, JSON parsing, optional LLM scoring request/response validation.
- Create `src/storage.rs`: `.content-score/` paths, SQLite schema, migrations, CRUD helpers.
- Create `src/prediction.rs`: prediction IDs, script hashing, prediction text hashing, Markdown rendering.
- Create `src/calibration.rs`: completed-sample analysis and conservative weight proposal logic.
- Create `src/upgrade.rs`: upgrade proposal and apply flow.
- Create `tests/cli_smoke.rs`: integration tests against a temp project directory.
- Modify `.gitignore`: ignore build artifacts and local runtime data.

---

### Task 1: Toolchain And Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `.gitignore`
- Test: `cargo test`

- [ ] **Step 1: Install user-local Rust if needed**

Run:

```bash
command -v cargo || curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
. "$HOME/.cargo/env"
cargo --version
rustc --version
```

Expected: `cargo` and `rustc` versions print successfully.

- [ ] **Step 2: Write the failing scaffold test**

Create `src/main.rs` with only an empty placeholder module and add this test:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn binary_name_is_content_score() {
        assert_eq!(env!("CARGO_PKG_NAME"), "content-score");
    }
}

fn main() {}
```

Run: `cargo test`

Expected: FAIL because `Cargo.toml` does not exist yet.

- [ ] **Step 3: Add Cargo scaffold**

Create `Cargo.toml`:

```toml
[package]
name = "content-score"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4", features = ["derive"] }
dirs = "5"
reqwest = { version = "0.12", default-features = false, features = ["blocking", "json", "rustls-tls"] }
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
tempfile = "3"
thiserror = "1"
toml = "0.8"
uuid = { version = "1", features = ["v4", "serde"] }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

Create `.gitignore`:

```gitignore
/target/
/.content-score/
/predictions/
```

- [ ] **Step 4: Verify scaffold passes**

Run: `cargo test`

Expected: PASS with one unit test.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/main.rs .gitignore
git commit -m "chore: scaffold Rust CLI"
```

---

### Task 2: Rubric And Score Core

**Files:**
- Create: `src/dimensions.rs`
- Create: `src/rubric.rs`
- Create: `src/score.rs`
- Modify: `src/main.rs`
- Test: unit tests in the three modules

- [ ] **Step 1: Write failing tests for dimensions, score validation, and composite**

Tests must cover:

```rust
assert_eq!(Dimension::all().len(), 7);
assert_eq!(Dimension::parse("ER").unwrap(), Dimension::Er);
assert!(Dimension::parse("BAD").is_err());
assert!(ScoreSet::from_pairs(vec![("ER", 6)]).is_err());
assert!(ScoreSet::from_pairs(vec![("ER", 3)]).is_err());
let scores = ScoreSet::from_pairs(vec![
    ("ER", 4), ("HP", 5), ("QL", 3), ("NA", 3), ("AB", 4), ("SR", 2), ("SAT", 1),
]).unwrap();
let rubric = Rubric::default_v0();
assert!((rubric.composite(&scores) - 6.285714).abs() < 0.0001);
```

Run: `cargo test dimensions rubric score`

Expected: FAIL because modules do not exist.

- [ ] **Step 2: Implement dimensions and score set**

Implement:

```rust
pub enum Dimension { Er, Hp, Ql, Na, Ab, Sr, Sat }
pub struct DimensionScore { pub score: u8, pub reason: String }
pub struct ScoreSet { pub scores: BTreeMap<Dimension, DimensionScore> }
```

Rules:

- every score must be `0..=5`
- all seven dimensions are required
- codes are parsed case-insensitively

- [ ] **Step 3: Implement default v0 rubric**

Implement default weights all equal to `1.0`, version `v0`, and:

```rust
weighted_sum / total_weight * 2.0
```

- [ ] **Step 4: Verify**

Run: `cargo test dimensions rubric score`

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/dimensions.rs src/rubric.rs src/score.rs src/main.rs
git commit -m "feat: add rubric scoring core"
```

---

### Task 3: Local Storage And Init Command

**Files:**
- Create: `src/storage.rs`
- Modify: `src/main.rs`
- Test: `tests/cli_smoke.rs`

- [ ] **Step 1: Write failing init smoke test**

The test should run the binary in a temp directory:

```rust
Command::cargo_bin("content-score")
    .unwrap()
    .current_dir(temp.path())
    .arg("init")
    .assert()
    .success()
    .stdout(predicate::str::contains("initialized"));

assert!(temp.path().join(".content-score/content.sqlite").exists());
assert!(temp.path().join(".content-score/rubric.toml").exists());
```

Run: `cargo test --test cli_smoke init_creates_local_project`

Expected: FAIL because `init` is not implemented.

- [ ] **Step 2: Implement storage schema**

Create `.content-score/` and SQLite tables:

- `rubric_versions(version, weights_json, active, created_at)`
- `candidates(id, text, score_json, composite, created_at, scored_at)`
- `score_runs(id, target_type, target_ref, rubric_version, scores_json, composite, created_at)`
- `predictions(id, script_path, script_hash, rubric_version, scores_json, composite, bet, bucket, prediction_hash, contaminated, created_at)`
- `retros(id, prediction_id, plays, likes, comments, shares, saves, top_comments, notes, contaminated, created_at)`
- `upgrade_proposals(id, from_version, to_version, weights_json, rationale, status, created_at, applied_at)`

Also write `.content-score/rubric.toml` with v0 weights.

- [ ] **Step 3: Implement `content-score init`**

Dispatch `init` in `main.rs`, call storage initialization, and print:

```text
content-score initialized at .content-score
active rubric: v0
```

- [ ] **Step 4: Verify**

Run: `cargo test --test cli_smoke init_creates_local_project`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/storage.rs src/main.rs tests/cli_smoke.rs
git commit -m "feat: initialize local score project"
```

---

### Task 4: Score And Candidate Commands

**Files:**
- Modify: `src/main.rs`
- Modify: `src/storage.rs`
- Modify: `src/score.rs`
- Test: `tests/cli_smoke.rs`

- [ ] **Step 1: Write failing CLI tests**

Add tests for:

```bash
content-score score scripts/foo.md --scores ER=4,HP=5,QL=3,NA=3,AB=4,SR=2,SAT=1
content-score candidates add "AI makes one-person companies possible"
content-score candidates score 1 --scores ER=3,HP=4,QL=3,NA=2,AB=5,SR=4,SAT=1
content-score candidates top
```

Assertions:

- score output contains `composite: 6.29 / 10`
- candidate add output contains `candidate #1`
- candidate top output contains the candidate text and `candidate_score`

Expected: FAIL because commands are missing.

- [ ] **Step 2: Implement `score`**

Read script file, parse `--scores`, validate via `ScoreSet`, compute composite, store a `score_runs` row, and print a seven-line score table plus composite.

- [ ] **Step 3: Implement candidate commands**

Implement:

- `candidates add <text>`
- `candidates score <id> --scores <pairs>`
- `candidates top`

Store candidate scores in `candidates.score_json` and `candidates.composite`.

- [ ] **Step 4: Verify**

Run: `cargo test --test cli_smoke score_and_candidates_work`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/storage.rs src/score.rs tests/cli_smoke.rs
git commit -m "feat: score scripts and candidates"
```

---

### Task 5: Prediction Integrity And Retro

**Files:**
- Create: `src/prediction.rs`
- Modify: `src/main.rs`
- Modify: `src/storage.rs`
- Test: `tests/cli_smoke.rs`

- [ ] **Step 1: Write failing tests**

Add a test that:

1. runs `init`
2. writes `scripts/foo.md`
3. runs `predict scripts/foo.md --scores ER=4,HP=5,QL=3,NA=3,AB=4,SR=2,SAT=1 --bet "strong hook, weak satire"`
4. asserts `predictions/<id>.md` exists
5. runs `retro <id> --plays 1200 --likes 80 --comments 12 --shares 4 --saves 9 --notes "solid base"`
6. asserts output contains `retro recorded`

Expected: FAIL because `predict` and `retro` are missing.

- [ ] **Step 2: Implement prediction creation**

Compute:

- `script_hash = sha256(script bytes)`
- prediction ID from date + first 12 chars of script hash
- prediction markdown body
- `prediction_hash = sha256(markdown body)`

Store DB row and write Markdown under `predictions/`.

- [ ] **Step 3: Implement retro hash check**

Before inserting retro, read the prediction markdown and compare hash with stored `prediction_hash`.

- match: `contaminated = false`
- mismatch: insert retro with `contaminated = true` and print an integrity warning

- [ ] **Step 4: Verify**

Run: `cargo test --test cli_smoke predict_and_retro_work`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/prediction.rs src/main.rs src/storage.rs tests/cli_smoke.rs
git commit -m "feat: add prediction and retro loop"
```

---

### Task 6: Calibration And Upgrade Flow

**Files:**
- Create: `src/calibration.rs`
- Create: `src/upgrade.rs`
- Modify: `src/main.rs`
- Modify: `src/storage.rs`
- Test: `tests/cli_smoke.rs`

- [ ] **Step 1: Write failing tests**

Add tests that create at least three prediction + retro samples with varied scores and plays, then run:

```bash
content-score calibrate
content-score upgrade --propose
content-score upgrade --apply 1
```

Assertions:

- `calibrate` output contains `samples: 3`
- `upgrade --propose` output contains `upgrade proposal #1`
- `upgrade --apply 1` output contains `active rubric: v1`

Expected: FAIL because commands are missing.

- [ ] **Step 2: Implement calibration**

For completed, uncontaminated samples:

- print sample count
- for each dimension, compare average plays when score is `>=4` vs `<4`
- mark dimensions with too few high or low samples as `insufficient data`
- print conservative recommendation text

- [ ] **Step 3: Implement upgrade proposal**

Generate v1 weights from current v0 weights using conservative deltas:

- if high-score avg plays is at least 1.5x low-score avg and both groups have samples, add `0.2`
- if high-score avg plays is less than `0.8x` low-score avg and both groups have samples, subtract `0.2`
- clamp weights to `0.5..=2.0`

Store proposal with status `proposed`.

- [ ] **Step 4: Implement upgrade apply**

Apply only a `proposed` proposal, insert new active rubric version, mark old active as inactive, and mark proposal `applied`.

- [ ] **Step 5: Verify**

Run: `cargo test --test cli_smoke calibrate_and_upgrade_work`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/calibration.rs src/upgrade.rs src/main.rs src/storage.rs tests/cli_smoke.rs
git commit -m "feat: calibrate and upgrade rubric"
```

---

### Task 7: LLM JSON Scorer And Final Verification

**Files:**
- Modify: `src/score.rs`
- Modify: `src/main.rs`
- Modify: `README.md`
- Test: unit tests in `score.rs`, all CLI tests

- [ ] **Step 1: Write failing tests for JSON scorer parsing**

Test that this JSON parses into a valid `ScoreSet`:

```json
{
  "ER": {"score": 4, "reason": "specific emotional recognition"},
  "HP": {"score": 5, "reason": "strong opening contrast"},
  "QL": {"score": 3, "reason": "one reusable line"},
  "NA": {"score": 3, "reason": "clear but simple arc"},
  "AB": {"score": 4, "reason": "broad creator audience"},
  "SR": {"score": 2, "reason": "weak social conflict"},
  "SAT": {"score": 1, "reason": "little irony"}
}
```

Also test that missing a dimension fails.

- [ ] **Step 2: Implement `--score-json` and optional `--llm`**

`--score-json <path>` reads strict JSON and validates it.

`--llm` reads these environment variables:

- `CONTENT_SCORE_LLM_ENDPOINT`
- `CONTENT_SCORE_LLM_API_KEY`
- `CONTENT_SCORE_LLM_MODEL`

It sends a compact prompt containing only the rubric and target text, then validates returned JSON with the same parser.

- [ ] **Step 3: Write README**

Document:

- install/build
- `init`
- candidate flow
- script score
- predict/retro
- calibrate/upgrade
- manual `--scores`
- strict JSON scorer
- optional LLM environment variables

- [ ] **Step 4: Run full verification**

Run:

```bash
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/score.rs src/main.rs README.md tests/cli_smoke.rs
git commit -m "feat: support JSON and LLM scoring"
```

---

## Self-Review

- Spec coverage: candidate scoring, script scoring, prediction hash integrity, manual retro, calibration, and confirmed upgrade are covered.
- Intentional first-version limits: no Douyin crawler, no TrendRadar, no web UI, no silent auto-upgrade, no nine-dimension rubric.
- Risk: local environment currently may not have Rust installed; Task 1 installs a user-local minimal toolchain before code verification.
