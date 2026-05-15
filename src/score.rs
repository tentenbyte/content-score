use crate::dimensions::Dimension;
use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DimensionScore {
    pub score: u8,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreSet {
    pub scores: BTreeMap<Dimension, DimensionScore>,
}

impl ScoreSet {
    #[cfg(test)]
    pub fn from_pairs(pairs: Vec<(&str, u8)>) -> Result<ScoreSet> {
        let mut scores = BTreeMap::new();
        for (code, score) in pairs {
            let dimension = Dimension::parse(code)?;
            scores.insert(
                dimension,
                DimensionScore {
                    score,
                    reason: "manual score".to_string(),
                },
            );
        }
        ScoreSet::new(scores)
    }

    pub fn new(scores: BTreeMap<Dimension, DimensionScore>) -> Result<ScoreSet> {
        for dimension in Dimension::all() {
            let Some(entry) = scores.get(dimension) else {
                return Err(anyhow!("missing score for {}", dimension.code()));
            };
            if entry.score > 5 {
                return Err(anyhow!(
                    "{} score must be between 0 and 5, got {}",
                    dimension.code(),
                    entry.score
                ));
            }
        }

        if scores.len() != Dimension::all().len() {
            return Err(anyhow!("score set must contain exactly seven dimensions"));
        }

        Ok(ScoreSet { scores })
    }

    pub fn from_code_map(scores: BTreeMap<String, DimensionScore>) -> Result<ScoreSet> {
        let mut parsed = BTreeMap::new();
        for (code, score) in scores {
            parsed.insert(Dimension::parse(&code)?, score);
        }
        ScoreSet::new(parsed)
    }

    pub fn from_json_str(input: &str) -> Result<ScoreSet> {
        let scores = serde_json::from_str::<BTreeMap<String, DimensionScore>>(input)?;
        ScoreSet::from_code_map(scores)
    }

    pub fn get(&self, dimension: Dimension) -> u8 {
        self.scores
            .get(&dimension)
            .map(|score| score.score)
            .expect("ScoreSet invariant violated: missing dimension")
    }

    pub fn by_code(&self) -> BTreeMap<String, DimensionScore> {
        self.scores
            .iter()
            .map(|(dimension, score)| (dimension.code().to_string(), score.clone()))
            .collect()
    }

    pub fn to_json_string(&self) -> Result<String> {
        Ok(serde_json::to_string(&self.by_code())?)
    }
}

pub fn parse_score_pairs(input: &str) -> Result<ScoreSet> {
    let mut scores = BTreeMap::new();
    for raw_pair in input.split(',') {
        let pair = raw_pair.trim();
        if pair.is_empty() {
            continue;
        }
        let Some((code, value)) = pair.split_once('=') else {
            return Err(anyhow!("score pair must look like ER=4, got {pair}"));
        };
        let dimension = Dimension::parse(code)?;
        let score = value
            .trim()
            .parse::<u8>()
            .map_err(|_| anyhow!("score for {} must be an integer", dimension.code()))?;
        scores.insert(
            dimension,
            DimensionScore {
                score,
                reason: "manual score".to_string(),
            },
        );
    }
    ScoreSet::new(scores)
}

pub fn load_score_json(path: &Path) -> Result<ScoreSet> {
    let input = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read score JSON at {}", path.display()))?;
    ScoreSet::from_json_str(&input)
}

pub fn score_with_llm(target_text: &str) -> Result<ScoreSet> {
    let endpoint = std::env::var("CONTENT_SCORE_LLM_ENDPOINT")
        .context("CONTENT_SCORE_LLM_ENDPOINT is required for --llm")?;
    let api_key = std::env::var("CONTENT_SCORE_LLM_API_KEY")
        .context("CONTENT_SCORE_LLM_API_KEY is required for --llm")?;
    let model = std::env::var("CONTENT_SCORE_LLM_MODEL")
        .context("CONTENT_SCORE_LLM_MODEL is required for --llm")?;
    let url = chat_completions_url(&endpoint);

    let body = json!({
        "model": model,
        "temperature": 0.2,
        "messages": [
            {
                "role": "system",
                "content": "Score Chinese content using exactly these dimensions: ER emotional resonance, HP hook potential, QL quotable lines, NA narrativity, AB audience breadth, SR social resonance, SAT satire depth. Return strict JSON only. Shape: {\"ER\":{\"score\":0-5,\"reason\":\"...\"}, ...}. Scores must be integers."
            },
            {
                "role": "user",
                "content": target_text
            }
        ]
    });

    let response: ChatCompletionResponse = Client::new()
        .post(url)
        .bearer_auth(api_key)
        .json(&body)
        .send()?
        .error_for_status()?
        .json()?;
    let content = response
        .choices
        .first()
        .map(|choice| choice.message.content.trim())
        .filter(|content| !content.is_empty())
        .ok_or_else(|| anyhow!("LLM response did not contain message content"))?;

    ScoreSet::from_json_str(content)
}

fn chat_completions_url(endpoint: &str) -> String {
    let trimmed = endpoint.trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1/chat/completions")
    }
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_complete_integer_score_sets() {
        assert!(ScoreSet::from_pairs(vec![
            ("ER", 4),
            ("HP", 5),
            ("QL", 3),
            ("NA", 3),
            ("AB", 4),
            ("SR", 2),
            ("SAT", 1),
        ])
        .is_ok());

        assert!(ScoreSet::from_pairs(vec![
            ("ER", 6),
            ("HP", 5),
            ("QL", 3),
            ("NA", 3),
            ("AB", 4),
            ("SR", 2),
            ("SAT", 1),
        ])
        .is_err());

        assert!(ScoreSet::from_pairs(vec![("ER", 3)]).is_err());
    }

    #[test]
    fn parses_score_pair_string() {
        let scores = parse_score_pairs("ER=4,HP=5,QL=3,NA=3,AB=4,SR=2,SAT=1").unwrap();
        assert_eq!(scores.get(crate::dimensions::Dimension::Hp), 5);
        assert!(parse_score_pairs("ER=4").is_err());
    }

    #[test]
    fn parses_strict_score_json_and_rejects_missing_dimension() {
        let json = r#"{
          "ER": {"score": 4, "reason": "specific emotional recognition"},
          "HP": {"score": 5, "reason": "strong opening contrast"},
          "QL": {"score": 3, "reason": "one reusable line"},
          "NA": {"score": 3, "reason": "clear but simple arc"},
          "AB": {"score": 4, "reason": "broad creator audience"},
          "SR": {"score": 2, "reason": "weak social conflict"},
          "SAT": {"score": 1, "reason": "little irony"}
        }"#;

        let scores = ScoreSet::from_json_str(json).unwrap();
        assert_eq!(scores.get(crate::dimensions::Dimension::Er), 4);

        let missing = r#"{"ER": {"score": 4, "reason": "x"}}"#;
        assert!(ScoreSet::from_json_str(missing).is_err());
    }
}
