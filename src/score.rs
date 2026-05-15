use crate::dimensions::Dimension;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
}
