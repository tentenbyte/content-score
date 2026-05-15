use crate::dimensions::Dimension;
use crate::score::ScoreSet;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rubric {
    pub version: String,
    pub weights: BTreeMap<Dimension, f64>,
}

impl Rubric {
    pub fn default_v0() -> Rubric {
        let weights = Dimension::all()
            .iter()
            .copied()
            .map(|dimension| (dimension, 1.0))
            .collect();

        Rubric {
            version: "v0".to_string(),
            weights,
        }
    }

    pub fn from_code_weights(version: String, weights: BTreeMap<String, f64>) -> Result<Rubric> {
        let mut parsed = BTreeMap::new();
        for (code, weight) in weights {
            parsed.insert(Dimension::parse(&code)?, weight);
        }

        Ok(Rubric {
            version,
            weights: parsed,
        })
    }

    pub fn composite(&self, scores: &ScoreSet) -> f64 {
        let weighted_sum = self
            .weights
            .iter()
            .map(|(dimension, weight)| scores.get(*dimension) as f64 * weight)
            .sum::<f64>();
        let total_weight = self.weights.values().sum::<f64>();

        weighted_sum / total_weight * 2.0
    }

    pub fn weights_by_code(&self) -> BTreeMap<String, f64> {
        self.weights
            .iter()
            .map(|(dimension, weight)| (dimension.code().to_string(), *weight))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::ScoreSet;

    #[test]
    fn default_v0_computes_composite_on_zero_to_ten_scale() {
        let scores = ScoreSet::from_pairs(vec![
            ("ER", 4),
            ("HP", 5),
            ("QL", 3),
            ("NA", 3),
            ("AB", 4),
            ("SR", 2),
            ("SAT", 1),
        ])
        .unwrap();
        let rubric = Rubric::default_v0();

        assert!((rubric.composite(&scores) - 6.285714).abs() < 0.0001);
    }
}
