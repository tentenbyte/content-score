# content-score

Local Rust CLI for content scoring, prediction logging, retros, calibration, and confirmed rubric upgrades.

`content-score` is an independent local CLI inspired by
[`XBuilderLAB/cheat-on-content`](https://github.com/XBuilderLAB/cheat-on-content).
Thanks to XBuilderLAB and the Cheat on Content creator for the core idea of
turning creator judgment into a measurable loop:

```text
score -> predict -> publish -> retro -> calibrate -> upgrade
```

This project does not vendor the original skill implementation. It rebuilds the
workflow as a small Rust CLI with local SQLite storage and an optional Codex
skill bridge.

## Build

```bash
cargo build
```

## Initialize

```bash
content-score init
```

This creates:

```text
.content-score/content.sqlite
.content-score/rubric.toml
predictions/
```

## Codex Skill

This repository includes the matching Codex skill:

```text
skills/content-score/
  SKILL.md
  agents/openai.yaml
  references/scoring.md
```

To install it locally from a clone:

```bash
mkdir -p "$HOME/.codex/skills"
ln -s "$(pwd)/skills/content-score" "$HOME/.codex/skills/content-score"
```

If `~/.codex/skills/content-score` already exists, move or remove the old local copy before linking. The skill is a workflow bridge; the Rust CLI remains the durable system of record.

## Score A Script

Manual scores:

```bash
content-score score scripts/foo.md \
  --scores ER=4,HP=5,QL=3,NA=3,AB=4,SR=2,SAT=1
```

Strict JSON scores:

```bash
content-score score scripts/foo.md --score-json score.json
```

JSON shape:

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

Optional LLM scoring uses an OpenAI-compatible chat-completions endpoint:

```bash
export CONTENT_SCORE_LLM_ENDPOINT="https://api.example.com"
export CONTENT_SCORE_LLM_API_KEY="..."
export CONTENT_SCORE_LLM_MODEL="model-name"
content-score score scripts/foo.md --llm
```

## Candidates

```bash
content-score candidates add "AI makes one-person companies possible"
content-score candidates score 1 --scores ER=3,HP=4,QL=3,NA=2,AB=5,SR=4,SAT=1
content-score candidates top
```

Candidate scores are prioritization signals, not predictions.

## Predict And Retro

```bash
content-score predict scripts/foo.md \
  --scores ER=4,HP=5,QL=3,NA=3,AB=4,SR=2,SAT=1 \
  --bet "strong hook, weak satire"

content-score retro <prediction-id> \
  --plays 1200 --likes 80 --comments 12 --shares 4 --saves 9 \
  --notes "solid base"
```

`predict` writes a Markdown file under `predictions/` and stores its hash. `retro` checks that hash before recording real performance. Edited prediction files are marked contaminated.

## Batch Retro Import

CSV:

```bash
content-score retro import douyin.csv
```

```csv
prediction_id,plays,likes,comments,shares,saves,top_comments,notes
2026-05-15_xxx,1200,80,12,4,9,"comment1|comment2","T+3"
```

JSON:

```bash
content-score retro import douyin.json
```

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
    "notes": "T+3"
  }
]
```

Import continues after row-level failures and prints imported, failed, and contaminated counts.

## Douyin Semi-Automatic Retro Fetch

The Douyin adapter can fetch metrics for a known prediction and import them as a retro:

```bash
content-score douyin doctor
content-score douyin login
content-score douyin fetch <prediction-id> <url-or-id>
```

Run these commands inside an initialized `content-score` project. `doctor` reports project/dependency status, `login` requires `.content-score/content.sqlite`, and the CLI passes the current project root to the adapter so `.auth/` and debug files stay local to that project.

`fetch` accepts a raw aweme id, a long Douyin video URL, or a `v.douyin.com` short link. By default it writes a JSON backup to `.content-score/imports/douyin-<prediction-id>.json` and imports that file through the normal retro import path.

Fetch options:

- `--no-import`: write the JSON backup but do not import it.
- `--dry-run`: run the adapter and validate its JSON output without importing it.
- `--replace`: replace an existing retro for the same prediction. Without this, duplicate retros are rejected.

Set up the Python adapter dependencies in the content project where you will run the command. Point `CONTENT_SCORE_REPO` at this repository clone:

```bash
export CONTENT_SCORE_REPO=/path/to/content-score
python3 -m venv .venv
source .venv/bin/activate
pip install -r "$CONTENT_SCORE_REPO/adapters/douyin-session/requirements.txt"
python -m playwright install chromium
```

For adapter development or tests, `CONTENT_SCORE_DOUYIN_ADAPTER=/path/to/cli.py` overrides the repository adapter path.

`content-score douyin login` stores the browser session under `.auth/`. Fetch failures may write captured diagnostic files under `.content-score/douyin-debug/`. Treat both paths as local-only data; do not commit them.

Live Douyin behavior depends on Playwright, Chromium, a valid user-authorized Douyin login, and Douyin's current Creator Center/public-page behavior. It may need maintenance when Douyin changes its pages or network responses.

## Calibrate And Upgrade

```bash
content-score calibrate
content-score upgrade --propose
content-score upgrade --apply 1
```

The first version only adjusts weights conservatively. It never silently changes the active rubric.

## Acknowledgements

- [`XBuilderLAB/cheat-on-content`](https://github.com/XBuilderLAB/cheat-on-content)
  for the scoring, blind-prediction, retro, and rubric-evolution loop that
  inspired this project.
- [`sansan0/TrendRadar`](https://github.com/sansan0/TrendRadar) as a useful
  reference direction for future trend-source integrations. `content-score`
  currently does not depend on TrendRadar code.

## License

MIT.
