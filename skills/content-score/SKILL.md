---
name: content-score
description: Use when the user is evaluating Douyin or other content ideas/scripts, preparing a pre-publication content bet, recording post-publication metrics, or asking what their content performance data implies.
---

# Content Score Skill

Coordinates the `content-score` Rust CLI for content scoring, prediction logs, retros, calibration, and rubric upgrades.

The CLI is the source of truth for state, validation, formulas, prediction hashes, and rubric versions. The skill decides workflow and writes scoring JSON when Codex is the scorer.

## Command resolution

Use the first available command prefix:

1. `content-score`
2. `$HOME/.cargo/bin/content-score`
3. `cargo run --manifest-path "$CONTENT_SCORE_REPO/Cargo.toml" --`
4. `cargo run --manifest-path /home/tt/content-score/Cargo.toml --` when that local repo exists

Run commands from the user's content project root so `.content-score/`, `predictions/`, and imported files are read/written in the right project.

## Top-level modes and rules

This skill has four top-level modes:

- **Candidate mode:** score rough ideas/titles before a full script exists.
- **Script mode:** score a script or write a pre-publication prediction from a script file.
- **Retro mode:** record or import post-publication metrics, including user-authorized Douyin fetches for a known prediction id.
- **Calibration mode:** inspect patterns, propose/apply rubric changes.

Rules:

- If `.content-score/content.sqlite` is missing, initialize first with `content-score init`.
- Durable actions must go through the CLI. Do not leave important scores, predictions, retros, or upgrades only in chat.
- Use `--score-json` by default when Codex scores content. Write the JSON under `.content-score/`.
- Use `--scores` only when exact numeric scores are supplied by the user or when doing a tiny deterministic smoke test.
- Use `--llm` only if the user explicitly requests the external LLM path and `CONTENT_SCORE_LLM_ENDPOINT`, `CONTENT_SCORE_LLM_API_KEY`, and `CONTENT_SCORE_LLM_MODEL` are configured.
- Never run `upgrade --apply <id>` unless the user explicitly confirms applying that exact proposal.
- Never invent retro metrics. If required values are missing, ask for them.
- Never write a blind prediction after the user has shared post-publication metrics for that same piece.
- For scoring and prediction, do not use actual plays, likes, comments, shares, saves, or prior result hints in the score JSON.
- Use Douyin fetch only for user-authorized Douyin data and an explicit known `prediction_id`; do not infer or match predictions automatically.

## When to use

- The user asks whether an idea/topic is worth writing.
- The user asks to score, compare, or diagnose a content script.
- The user is about to publish and wants a prediction/bet.
- The user brings back post-publication metrics and wants a retro.
- The user asks what patterns are emerging from past posts.
- The user asks whether the rubric/weights should change.
- The user asks to fetch Douyin metrics for a specific known prediction id and URL/aweme id.

## When not to use

- The user is only asking conceptually how content scoring works.
- The user wants broad content coaching with no idea, script, metrics, or project state.
- The user wants hotspot crawling, TrendRadar integration, broad platform automation, or unsupported scraping.
- The task is to modify the Rust CLI code itself.

## Decision tree

Think about two questions: what artifact exists, and whether post-publication metrics have been seen.

1. Confirm project state:
   - If `.content-score/content.sqlite` is missing, run init.
2. If the user provided a rough idea/title only:
   - Use candidate mode.
3. If the user provided a script file:
   - If they want feedback/diagnosis only, use script score mode.
   - If they are about to publish and no metrics have been seen, use prediction mode.
4. If the user provided real metrics:
   - Use retro mode. Do not write a blind prediction.
   - If they provide a CSV/JSON file, use batch retro import.
5. If the user asks to fetch Douyin metrics:
   - Require an explicit `prediction_id` and Douyin URL or aweme id.
   - Use Douyin retro fetch mode. Do not create or infer a prediction.
6. If the user asks for patterns:
   - Use calibration mode.
7. If the user asks to change weights:
   - Run `upgrade --propose`; apply only after confirmation.

## Score JSON policy

Write score JSON files under `.content-score/` with stable names:

- candidate: `.content-score/candidate-<id>-score.json`
- script score: `.content-score/<script-stem>-score.json`
- prediction: reuse the same script score JSON when possible

Do not overwrite an existing score JSON unless it is for the same artifact and the user expects a rescore. Otherwise add a suffix such as `-v2`.

Strict JSON shape and detailed dimension guidance live in [references/scoring.md](references/scoring.md). Read it before writing score JSON unless the user supplied exact scores.

## Workflows

In examples below, `content-score` means the resolved command prefix from "Command resolution".

### Candidate mode

Use when the input is an idea, title, or angle rather than a full script.

```bash
content-score candidates add "<idea text>"
content-score candidates score <id> --score-json .content-score/candidate-<id>-score.json
content-score candidates top
```

Candidate scores are prioritization signals, not predictions. Report why the top candidate is worth drafting, and what must be sharpened before it becomes a script.

### Script score mode

Use when the user wants a score without committing to a prediction.

```bash
content-score score scripts/foo.md --score-json .content-score/foo-score.json
```

After the command, summarize composite, strongest dimensions, weakest dimensions, and one concrete next edit if useful.

### Prediction mode

Use only for a publishable draft before post-publication metrics are known.

```bash
content-score predict scripts/foo.md \
  --score-json .content-score/foo-score.json \
  --bet "HP strong, SR weak; expect baseline-to-hit range"
```

The bet must be concise and falsifiable. Avoid generic "this should do well" language.

### Retro mode

Required metrics:

- plays
- likes
- comments
- shares
- saves

Optional but useful:

- notes
- top comments

```bash
content-score retro <prediction-id> \
  --plays 1200 --likes 80 --comments 12 --shares 4 --saves 9 \
  --notes "solid base"
```

If the CLI reports an integrity warning, explain that the prediction file changed and the retro may be excluded from calibration.

For batch import:

```bash
content-score retro import douyin.csv
content-score retro import douyin.json
```

CSV requires:

```csv
prediction_id,plays,likes,comments,shares,saves,top_comments,notes
```

JSON must be an array of objects with the same metrics. `top_comments` may be an array in JSON.

After import, report imported, failed, and contaminated counts. If failed rows exist, mention the row-level errors and do not imply the whole import succeeded.

### Douyin retro fetch mode

Use only when the user provides a known `prediction_id` and a Douyin raw aweme id, long video URL, or short link.

```bash
content-score douyin doctor
content-score douyin login
content-score douyin fetch <prediction-id> <url-or-aweme-id>
```

Run `doctor` first when the environment is unknown. Run `login` only when the user needs to establish or refresh a user-authorized Douyin session.

`fetch` writes `.content-score/imports/douyin-<prediction-id>.json` and imports it by default through the standard retro import path. Default duplicate behavior rejects a second retro for the same prediction.

Useful options:

- `--no-import`: write the JSON backup without recording a retro.
- `--dry-run`: run and validate fetch output without recording a retro.
- `--replace`: replace an existing retro for the same prediction.

If Playwright, Chromium, login state, or Douyin page behavior blocks the fetch, report the exact failure and do not fabricate metrics.

### Calibration and upgrade mode

```bash
content-score calibrate
content-score upgrade --propose
```

Only apply after explicit confirmation:

```bash
content-score upgrade --apply <proposal-id>
```

## CLI unavailable

If all command-resolution options fail, report that the CLI is unavailable and do not fake a persisted result. Recommend installing from the repository:

```bash
cargo install --path /path/to/content-score --force
```

## Output policy

After running the CLI, report only high-signal results:

- command purpose, if not obvious
- candidate score or composite
- strongest and weakest dimensions
- prediction id/path, when created
- retro integrity warning, if any
- import counts and failed rows, for batch retro import
- Douyin JSON backup path and whether import occurred
- next recommended command

Do not present unpersisted chat-only scores as if they were logged in the system.

## Common mistakes

- Running from the CLI repo instead of the content project root.
- Treating candidate scores as predictions.
- Applying upgrades immediately after proposing them.
- Predicting after the user already supplied metrics.
- Scoring with hidden knowledge from prior performance instead of only the current idea/script.
- Forgetting to write score JSON before calling `--score-json`.
- Guessing which prediction a Douyin URL belongs to instead of requiring the explicit `prediction_id`.
