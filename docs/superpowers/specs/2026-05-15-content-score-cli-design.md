# content-score CLI Design

## Goal

Build a pure local Rust CLI that turns content judgment into a repeatable scoring and calibration loop. The tool is not a Codex/Claude skill and does not provide a web UI.

The first usable loop is:

```text
candidate-score -> score -> predict -> retro -> calibrate -> upgrade
```

## Scope

The tool implements the seven starter dimensions from `cheat-on-content`:

| Code | Name | Meaning |
|---|---|---|
| ER | Emotional Resonance | Whether the idea or script carries concrete emotional recognition. |
| HP | Hook Potential | Whether the opening can hold attention. |
| QL | Quotable Lines | Whether there are reusable lines or concepts. |
| NA | Narrativity | Whether the script has a clear arc. |
| AB | Audience Breadth | How broad the likely audience is. |
| SR | Social Resonance | Whether the piece touches a shared social pattern. |
| SAT | Satire Depth | Whether satire, irony, parody, or self-reference is doing work. |

Scores are integers from 0 to 5. The v0 composite formula is:

```text
composite = (ER + HP + QL + NA + AB + SR + SAT) / 7 * 2.0
```

The score range is 0 to 10.

## CLI Commands

```text
content-score init
content-score candidates add <text>
content-score candidates score
content-score candidates top
content-score score <script.md>
content-score predict <script.md>
content-score retro <prediction-id>
content-score list
content-score calibrate
content-score upgrade --propose
content-score upgrade --apply <upgrade-id>
```

## Data Model

Local project state lives under `.content-score/`.

```text
.content-score/
  config.toml
  rubric.toml
  content.sqlite
predictions/
```

SQLite stores candidates, scripts, score runs, predictions, retros, rubric versions, upgrade proposals, and applied upgrades. Markdown prediction files are also written for human-readable audit history.

## Scoring

Rust owns deterministic work:

- loading rubric dimensions and weights
- validating that all scores are integers in `0..=5`
- computing composite scores
- writing immutable prediction records
- storing retros and calibration data
- proposing and applying rubric version changes

Semantic scoring is pluggable. The first implementation supports:

- `manual`: prompts the user for seven dimension scores and reasons
- `llm`: calls an API-compatible model provider and requires strict JSON output

The CLI validates LLM output and rejects malformed or incomplete scoring results.

## Candidate Scoring

Candidate scoring ranks ideas before a full script exists. It uses the same seven dimensions, but the interpretation is lighter:

- ER: emotional entry potential
- HP: headline/opening potential
- QL: concept or line potential
- NA: ability to expand into an arc
- AB: likely audience breadth
- SR: shared social pattern
- SAT: suitability for irony or parody

Candidate scores are prioritization signals, not predictions.

## Prediction Discipline

`predict` writes a pre-publication prediction record containing:

- script path and hash
- rubric version
- seven dimension scores
- composite score
- plain-language bet
- optional bucket estimate
- prediction content hash

`retro` checks that the saved prediction hash still matches before recording real performance. If the prediction was edited, the retro is marked contaminated and excluded from calibration by default.

## Retro And Calibration

`retro` records actual performance manually:

- plays
- likes
- comments
- shares
- saves
- top comments or notes

`calibrate` analyzes completed samples and reports:

- which dimensions correlate with stronger performance
- which high-scored dimensions failed to predict outcomes
- where sample count is too low
- whether a rubric upgrade is justified

Calibration output is advisory until an upgrade is explicitly applied.

## Upgrade

`upgrade --propose` creates a versioned proposal with changed weights or notes. It does not apply changes.

`upgrade --apply <upgrade-id>` requires an existing proposal and writes a new rubric version. The old version remains available for audit and future comparison.

First version upgrades are conservative: adjust weights only. Adding or removing dimensions is out of scope for the first implementation.

## Non-Goals

The first implementation does not include:

- Douyin auto-crawling
- TrendRadar integration
- web UI
- Codex/Claude skill installation
- cross-model audit
- automatic silent rubric changes
- nine-dimension rubric variants

## Testing

The implementation should include tests for:

- composite score calculation
- score validation
- rubric version parsing
- prediction hash integrity
- candidate ranking
- calibration with small sample counts
- upgrade proposal and application flow
