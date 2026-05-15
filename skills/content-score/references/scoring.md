# Content Score Scoring Reference

Use this reference when Codex writes a score JSON file for `content-score`.

## JSON schema

All seven dimensions are required. Scores are integers `0..5`. Reasons are short strings.

```json
{
  "ER": {"score": 4, "reason": "specific emotional recognition"},
  "HP": {"score": 5, "reason": "strong opening contrast"},
  "QL": {"score": 3, "reason": "one reusable line"},
  "NA": {"score": 3, "reason": "clear but simple arc"},
  "AB": {"score": 4, "reason": "broad audience"},
  "SR": {"score": 2, "reason": "weak social conflict"},
  "SAT": {"score": 1, "reason": "little irony"}
}
```

## Scoring rule

Use only the current idea/script text and general rubric meaning. Do not use post-publication metrics, prior performance hints, or user excitement as evidence.

Each reason should cite a concrete word, scene, claim, structure, or absence from the idea/script.

For candidate scoring, phrase reasons as potential: "can become", "has room for", "needs a concrete scene". Do not imply a candidate score predicts plays.

## Dimensions

### ER - Emotional Resonance

Question: does the content create concrete emotional recognition?

- `0`: no emotional stake
- `3`: recognizable but generic feeling
- `5`: sharp, specific, slightly uncomfortable self-recognition

Look for concrete situations, shame/pride/fear/desire, confessions, or phrases a viewer might say "this is me" about.

### HP - Hook Potential

Question: can the opening hold attention quickly?

- `0`: generic opening or slow setup
- `3`: clear promise, contrast, or question
- `5`: vivid image, contradiction, or immediate unresolved tension

For scripts, judge the first lines. For candidates, judge whether a strong opening is naturally available.

### QL - Quotable Lines

Question: are there reusable lines or concepts?

- `0`: purely explanatory
- `3`: one memorable phrase or framing
- `5`: multiple lines that can survive outside the video

Look for compressible concepts, screenshots, comments repeating a phrase, or sentence-level sharpness.

### NA - Narrativity

Question: is there an arc rather than a flat list?

- `0`: scattered points
- `3`: clear progression
- `5`: strong setup, turn, and payoff

For opinion videos, an argument arc counts even without a literal story.

### AB - Audience Breadth

Question: how many viewers can plausibly feel addressed?

- `0`: narrow insider niche
- `3`: medium audience with context requirement
- `5`: broad life/work/social situation with low context barrier

Broad does not mean viral. Broad topics can still be weak if ER/HP/QL are low.

### SR - Social Resonance

Question: does it name a shared social pattern?

- `0`: purely personal or private
- `3`: recognizable social phenomenon
- `5`: names a structural pattern viewers know but lack language for

Look for workplace, family, class, gender, platform, AI, career, education, or social-status patterns.

### SAT - Satire Depth

Question: does irony, parody, self-reference, or contrast carry meaning?

- `0`: sincere direct statement
- `3`: one layer of irony or format play
- `5`: layered satire, parody, or self-aware inversion

Do not penalize sincere formats too harshly: if satire is irrelevant, low SAT can be fine.

## Reason examples

Good:

- `"first line gives a concrete PPT-cat scene"`
- `"topic is AI anxiety but lacks social conflict"`
- `"has one reusable phrase but no repeated motif"`

Bad:

- `"will probably go viral"`
- `"user likes this topic"`
- `"last similar video did 5w"`
- `"good content"`
