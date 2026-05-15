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
}
