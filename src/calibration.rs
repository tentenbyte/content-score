use crate::dimensions::Dimension;
use crate::rubric::Rubric;
use crate::score::ScoreSet;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct CompletedSample {
    pub scores: ScoreSet,
    pub plays: i64,
}

#[derive(Debug, Clone)]
pub struct DimensionAnalysis {
    pub dimension: Dimension,
    pub high_count: usize,
    pub low_count: usize,
    pub high_avg: Option<f64>,
    pub low_avg: Option<f64>,
    pub ratio: Option<f64>,
    pub recommendation: String,
}

#[derive(Debug, Clone)]
pub struct CalibrationReport {
    pub sample_count: usize,
    pub dimensions: Vec<DimensionAnalysis>,
}

impl CalibrationReport {
    pub fn render(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("samples: {}", self.sample_count));
        for item in &self.dimensions {
            match (item.high_avg, item.low_avg, item.ratio) {
                (Some(high), Some(low), Some(ratio)) => lines.push(format!(
                    "{} high_avg={:.0} low_avg={:.0} ratio={:.2} {}",
                    item.dimension.code(),
                    high,
                    low,
                    ratio,
                    item.recommendation
                )),
                _ => lines.push(format!(
                    "{} insufficient data high={} low={}",
                    item.dimension.code(),
                    item.high_count,
                    item.low_count
                )),
            }
        }
        lines.join("\n")
    }
}

pub fn analyze(samples: &[CompletedSample]) -> CalibrationReport {
    let mut dimensions = Vec::new();
    for dimension in Dimension::all() {
        let mut high = Vec::new();
        let mut low = Vec::new();
        for sample in samples {
            if sample.scores.get(*dimension) >= 4 {
                high.push(sample.plays as f64);
            } else {
                low.push(sample.plays as f64);
            }
        }

        let high_avg = average(&high);
        let low_avg = average(&low);
        let ratio = match (high_avg, low_avg) {
            (Some(high), Some(low)) if low > 0.0 => Some(high / low),
            _ => None,
        };
        let recommendation = match ratio {
            Some(value) if value >= 1.5 => "suggest weight +0.2".to_string(),
            Some(value) if value < 0.8 => "suggest weight -0.2".to_string(),
            Some(_) => "keep weight".to_string(),
            None => "insufficient data".to_string(),
        };

        dimensions.push(DimensionAnalysis {
            dimension: *dimension,
            high_count: high.len(),
            low_count: low.len(),
            high_avg,
            low_avg,
            ratio,
            recommendation,
        });
    }

    CalibrationReport {
        sample_count: samples.len(),
        dimensions,
    }
}

pub fn propose_weights(current: &Rubric, report: &CalibrationReport) -> BTreeMap<String, f64> {
    let mut weights = current.weights_by_code();
    for item in &report.dimensions {
        let Some(ratio) = item.ratio else {
            continue;
        };
        let entry = weights
            .entry(item.dimension.code().to_string())
            .or_insert(1.0);
        if ratio >= 1.5 {
            *entry = (*entry + 0.2).min(2.0);
        } else if ratio < 0.8 {
            *entry = (*entry - 0.2).max(0.5);
        }
    }
    weights
}

fn average(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::ScoreSet;

    #[test]
    fn analyzes_high_vs_low_dimension_performance() {
        let samples = vec![
            CompletedSample {
                scores: ScoreSet::from_pairs(vec![
                    ("ER", 5),
                    ("HP", 5),
                    ("QL", 3),
                    ("NA", 3),
                    ("AB", 4),
                    ("SR", 2),
                    ("SAT", 1),
                ])
                .unwrap(),
                plays: 5000,
            },
            CompletedSample {
                scores: ScoreSet::from_pairs(vec![
                    ("ER", 2),
                    ("HP", 3),
                    ("QL", 2),
                    ("NA", 3),
                    ("AB", 4),
                    ("SR", 5),
                    ("SAT", 2),
                ])
                .unwrap(),
                plays: 900,
            },
        ];
        let report = analyze(&samples);
        let er = report
            .dimensions
            .iter()
            .find(|item| item.dimension == Dimension::Er)
            .unwrap();

        assert_eq!(report.sample_count, 2);
        assert!(er.ratio.unwrap() > 5.0);
    }
}
