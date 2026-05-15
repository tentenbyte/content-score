# Douyin Adapter Design

## Goal

Add the first Douyin data-acquisition path for `content-score`.

The feature should let the user provide a known `prediction_id` and a Douyin video URL or `aweme_id`, fetch that video's post-publication performance data with a local Playwright/Chromium adapter, write a standard retro import JSON file, and by default import that retro into the existing calibration loop.

The target loop is:

```text
prediction_id + Douyin URL
  -> local Playwright fetch
  -> .content-score/imports/douyin-<prediction-id>.json
  -> content-score retro import
  -> calibration pool
```

The core value is not broad crawling. The core value is reducing manual retro entry for content the user already published and already predicted.

## Non-Goals / Boundaries

First version does not do:

- automatic matching between predictions and Douyin videos
- bulk fetching recent videos
- hotspot/trend discovery
- keyword search
- scraping accounts or videos the user is not authorized to access
- CAPTCHA bypass, login bypass, or account restriction bypass
- direct writes from the adapter into SQLite
- silent automatic installation of Playwright or Chromium
- changing the scoring rubric or calibration algorithm
- replacing the existing CSV/JSON retro import path

The adapter may use the user's authenticated browser session to access data available to that user in Douyin Creator Center or the normal video page. If Douyin requires interactive verification, the user handles it in the browser; the tool should not attempt to bypass it.

## User Preferences

Confirmed preferences:

- Use an external Python Playwright/Chromium adapter for Douyin fetching.
- Keep Rust CLI as the orchestrator and source of truth for validation and persistence.
- Support three input forms:
  - raw `aweme_id`
  - `https://www.douyin.com/video/<aweme_id>`
  - `https://v.douyin.com/...` short links
- Require the user to manually provide `prediction_id`; do not infer it.
- Default `fetch` behavior should import the retro automatically after a successful fetch.
- Always keep the generated JSON backup under `.content-score/imports/`.
- Provide `--no-import` to fetch and write JSON without importing.
- Provide `--dry-run` to validate resolution/fetch/output without writing retro data.
- Duplicate retro behavior:
  - default: refuse to import when the `prediction_id` already has a retro
  - `--replace`: delete the old retro for that prediction and import the new one
  - no append mode in the first version
- Dependency behavior:
  - provide `content-score douyin doctor`
  - do not silently install Python packages or Chromium
  - print exact setup commands when dependencies are missing
- Store auth/session data in the user's content project, not in the repository.
- Do not commit auth/session/debug files.

## Commands

### Doctor

```bash
content-score douyin doctor
```

Checks the current content project for:

- `.content-score/content.sqlite`
- Python availability
- Playwright Python package availability
- Chromium browser availability for Playwright
- `.auth/` login-state directory
- adapter source path availability
- `.gitignore` coverage for `.auth/` and debug artifacts

`doctor` is informational. It must not install dependencies or mutate external auth state.

### Login

```bash
content-score douyin login
```

Runs the adapter login flow from the current content project root.

Expected behavior:

- opens Chromium through Playwright
- navigates to Douyin Creator Center
- lets the user scan QR code or complete interactive login
- persists browser session under `.auth/`
- exits successfully after login state is detected

### Fetch

```bash
content-score douyin fetch <prediction-id> <douyin-url-or-aweme-id>
content-score douyin fetch <prediction-id> <douyin-url-or-aweme-id> --no-import
content-score douyin fetch <prediction-id> <douyin-url-or-aweme-id> --dry-run
content-score douyin fetch <prediction-id> <douyin-url-or-aweme-id> --replace
```

Default fetch behavior:

```text
resolve input -> fetch Douyin metrics -> write JSON -> import retro
```

`--no-import` writes JSON only.

`--dry-run` resolves and fetches, prints a summary, and should not write retro data. It may write a temporary or clearly marked debug JSON only if needed for diagnostics.

`--replace` is valid only when importing. It replaces the existing retro for the same `prediction_id`. Combining `--replace` with `--no-import` or `--dry-run` should fail as an invalid option combination.

## Architecture

### Rust CLI Layer

Rust owns:

- CLI parsing
- project-state checks
- adapter command invocation
- path selection
- duplicate retro checks
- replace semantics
- JSON handoff to existing import logic
- final user-facing summaries and errors

New likely modules:

- `src/douyin.rs`: Douyin command orchestration, input resolution, adapter invocation, import handoff.
- `src/retro_import.rs`: may gain reusable import options for duplicate/replace behavior.
- `src/storage.rs`: may gain retro existence and deletion helpers.

### Python Adapter Layer

Python owns:

- Playwright persistent Chromium session
- Douyin Creator Center login flow
- short-link resolution when Rust delegates it
- page navigation
- response capture
- metric normalization
- top-comment extraction
- normalized JSON output

Adapter location:

```text
adapters/douyin-session/
  requirements.txt
  crawler.py
  normalize.py
  cli.py
```

The exact file split may change during implementation, but the adapter must expose stable commands callable by Rust:

```bash
python adapters/douyin-session/cli.py doctor
python adapters/douyin-session/cli.py login
python adapters/douyin-session/cli.py fetch <aweme-id-or-url> --output <path>
```

### Data Boundary

The adapter must not write SQLite. It writes normalized JSON only.

Rust imports that JSON through the same internal path as:

```bash
content-score retro import <file.json>
```

This keeps prediction hash checks, contaminated marking, validation, and calibration eligibility in one place.

## Data Flow

1. User runs:

```bash
content-score douyin fetch 2026-05-15_xxx https://v.douyin.com/abc123/
```

2. Rust validates that:

- current directory is a content-score project
- prediction exists
- duplicate retro policy allows the requested action
- adapter path and Python command are available

3. Rust or adapter resolves the input into an `aweme_id`.

4. Python adapter uses Playwright and the user's `.auth/` session to fetch available metrics.

5. Adapter writes normalized JSON to:

```text
.content-score/imports/douyin-2026-05-15_xxx.json
```

6. JSON shape:

```json
[
  {
    "prediction_id": "2026-05-15_xxx",
    "plays": 1200,
    "likes": 80,
    "comments": 12,
    "shares": 4,
    "saves": 9,
    "top_comments": ["comment1", "comment2"],
    "notes": "douyin aweme_id=123456789 fetched_at=2026-05-15T12:00:00Z"
  }
]
```

7. Rust imports the JSON unless `--no-import` or `--dry-run` was supplied.

8. CLI prints a concise summary:

```text
aweme_id: 123456789
title: ...
plays: 1200
likes: 80
comments: 12
shares: 4
saves: 9
top_comments: 2
json: .content-score/imports/douyin-2026-05-15_xxx.json
imported: yes
```

## Error Handling

### Missing Content Project

If `.content-score/content.sqlite` is missing:

- fail before running Playwright
- tell the user to run `content-score init`

### Missing Python / Playwright / Chromium

`doctor` and failed `login/fetch` should print exact remediation commands, for example:

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r /path/to/content-score/adapters/douyin-session/requirements.txt
playwright install chromium
```

### Missing Or Expired Login

If `.auth/` is missing or Creator Center redirects to login during fetch:

- fail without importing
- tell the user to run `content-score douyin login`

### Short Link Resolution Failure

If `https://v.douyin.com/...` cannot be resolved:

- fail before fetch
- tell the user to paste a full `douyin.com/video/<id>` URL or raw `aweme_id`

### Metrics Missing

If required metrics are missing:

- do not import
- keep any debug output under `.content-score/douyin-debug/`
- explain which required field was missing

Required metrics:

- plays
- likes
- comments
- shares
- saves

Top comments are optional. If comments cannot be fetched but counts are available, import may continue with empty `top_comments` and a warning.

### Duplicate Retro

Default:

- fail before launching the adapter if a retro already exists for `prediction_id`
- do not fetch or import unless the user chooses an explicit mode
- tell the user to rerun with `--replace` if they intend to overwrite

With `--no-import`:

- allow fetching even when a retro already exists, because no database write occurs
- write JSON so the user can inspect the latest Douyin data

With `--replace`:

- delete existing retro rows for that `prediction_id`
- import the newly fetched JSON
- print that replacement occurred

If a duplicate is discovered only after adapter fetch because of a race or stale preflight state, keep the generated JSON and fail without importing unless `--replace` was supplied.

### Import Failure

If adapter fetch succeeds but `retro import` fails:

- keep JSON
- print the import failure
- do not claim the sample entered calibration

## Storage And Git Hygiene

Generated user-project paths:

```text
.auth/
.content-score/imports/
.content-score/douyin-debug/
```

`.auth/` contains session credentials and must never be committed.

Debug artifacts may contain screenshots, URLs, or page response traces and must not be committed.

Implementation should ensure the generated project `.gitignore` covers:

```text
.auth/
.content-score/douyin-debug/
```

The repository `.gitignore` should continue to exclude generated local project state.

## Acceptance Criteria

The feature is complete when:

1. `content-score douyin doctor` reports dependency/project/login status without installing anything.
2. `content-score douyin login` opens Chromium and can persist login state under the current content project's `.auth/`.
3. `content-score douyin fetch <prediction-id> <aweme_id>` works for raw IDs.
4. `content-score douyin fetch <prediction-id> https://www.douyin.com/video/<id>` works for long links.
5. `content-score douyin fetch <prediction-id> https://v.douyin.com/...` resolves short links or fails with a clear message.
6. A successful fetch writes `.content-score/imports/douyin-<prediction-id>.json`.
7. The JSON contains `prediction_id`, `plays`, `likes`, `comments`, `shares`, `saves`, and `top_comments`.
8. Default fetch imports the retro into SQLite through the existing import path.
9. `--no-import` writes JSON without importing.
10. `--dry-run` does not write retro data.
11. Existing retro for the same prediction is rejected by default.
12. `--replace` overwrites the old retro for the same prediction.
13. Missing dependency, missing login, invalid link, missing prediction, missing required metrics, and import failure all produce clear errors and do not create incorrect retro rows.
14. Existing tests still pass:

```bash
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings
```

15. New Rust tests cover command parsing, duplicate/replace import behavior, JSON handoff, and link/ID resolution where network is not required.
16. New adapter tests cover normalization from representative captured payload fixtures without requiring live Douyin access.

## Testing Plan

### Rust Tests

Add CLI smoke tests for:

- `douyin doctor` in a missing-dependency or mocked-adapter scenario
- `douyin fetch` invoking a fake adapter that writes valid JSON
- default import path
- `--no-import`
- duplicate rejection
- `--replace`
- raw ID and long URL parsing

Use a fake adapter or environment override so tests do not require real Douyin, Python Playwright, or network access.

### Adapter Tests

Add fixture-based Python tests for:

- metric normalization
- top-comment normalization
- missing required metric detection
- short-link or URL parsing helpers where no live network is needed

Live Douyin testing remains manual because it depends on login state and current Douyin page/API behavior.

### Manual Verification

Manual end-to-end verification requires:

1. `content-score init` in a scratch content project.
2. Create a prediction.
3. Install Playwright dependencies in that project.
4. Run `content-score douyin login`.
5. Run `content-score douyin fetch <prediction-id> <real-douyin-url>`.
6. Confirm JSON exists.
7. Confirm retro exists and `content-score calibrate` sees the sample.

## Risks

- Douyin page/API structure may change and require adapter maintenance.
- Creator Center data may differ from public video-page data.
- Login sessions may expire unexpectedly.
- Comments are less reliable than headline metrics; top comments should be treated as helpful context, not a required calibration signal.
- Default auto-import is convenient but increases the cost of wrong links, so duplicate rejection and `--replace` must be implemented with care.

## Open Implementation Notes

- Prefer project-local virtualenv Python at `.venv/bin/python` when present, then `python3`.
- Add an environment override such as `CONTENT_SCORE_DOUYIN_ADAPTER` for tests and local debugging.
- Keep adapter stdout/stderr visible enough for diagnosis, but make final Rust summary concise.
- Avoid adding Playwright as a Rust dependency; let Python own browser automation.
