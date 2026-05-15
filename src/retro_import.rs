use crate::prediction;
use crate::storage;
use anyhow::{anyhow, Context, Result};
use rusqlite::Connection;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Default)]
pub struct ImportSummary {
    pub imported: usize,
    pub failed: usize,
    pub contaminated: usize,
    pub failures: Vec<ImportFailure>,
}

#[derive(Debug)]
pub struct ImportFailure {
    pub row_number: usize,
    pub prediction_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ImportOptions {
    pub replace_existing: bool,
}

#[derive(Debug, Clone)]
struct ImportRow {
    prediction_id: String,
    plays: i64,
    likes: i64,
    comments: i64,
    shares: i64,
    saves: i64,
    top_comments: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CsvImportRow {
    prediction_id: String,
    plays: i64,
    likes: i64,
    comments: i64,
    shares: i64,
    saves: i64,
    top_comments: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JsonImportRow {
    prediction_id: String,
    plays: i64,
    likes: i64,
    comments: i64,
    shares: i64,
    saves: i64,
    #[serde(default)]
    top_comments: Option<Vec<String>>,
    notes: Option<String>,
}

pub fn import_file(root: &Path, conn: &Connection, path: &Path) -> Result<ImportSummary> {
    import_file_with_options(root, conn, path, ImportOptions::default())
}

pub fn import_file_with_options(
    root: &Path,
    conn: &Connection,
    path: &Path,
    options: ImportOptions,
) -> Result<ImportSummary> {
    let rows = match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("json") => read_json_rows(path)?,
        _ => read_csv_rows(path)?,
    };

    let mut summary = ImportSummary::default();
    for (index, row) in rows.into_iter().enumerate() {
        let row_number = index + 1;
        let prediction_id = row.prediction_id.clone();
        match import_row(root, conn, row, options) {
            Ok(contaminated) => {
                summary.imported += 1;
                if contaminated {
                    summary.contaminated += 1;
                }
            }
            Err(error) => {
                summary.failed += 1;
                summary.failures.push(ImportFailure {
                    row_number,
                    prediction_id,
                    error: error.to_string(),
                });
            }
        }
    }

    Ok(summary)
}

fn read_csv_rows(path: &Path) -> Result<Vec<ImportRow>> {
    let mut reader = csv::Reader::from_path(path)
        .with_context(|| format!("failed to open CSV import file {}", path.display()))?;
    let mut rows = Vec::new();
    for record in reader.deserialize::<CsvImportRow>() {
        let row = record?;
        rows.push(ImportRow {
            prediction_id: row.prediction_id,
            plays: row.plays,
            likes: row.likes,
            comments: row.comments,
            shares: row.shares,
            saves: row.saves,
            top_comments: row.top_comments.filter(|value| !value.trim().is_empty()),
            notes: row.notes.filter(|value| !value.trim().is_empty()),
        });
    }
    Ok(rows)
}

fn read_json_rows(path: &Path) -> Result<Vec<ImportRow>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read JSON import file {}", path.display()))?;
    let rows = serde_json::from_str::<Vec<JsonImportRow>>(&content)?;
    Ok(rows
        .into_iter()
        .map(|row| ImportRow {
            prediction_id: row.prediction_id,
            plays: row.plays,
            likes: row.likes,
            comments: row.comments,
            shares: row.shares,
            saves: row.saves,
            top_comments: row
                .top_comments
                .map(|comments| comments.join(" | "))
                .filter(|value| !value.trim().is_empty()),
            notes: row.notes.filter(|value| !value.trim().is_empty()),
        })
        .collect())
}

fn import_row(
    root: &Path,
    conn: &Connection,
    row: ImportRow,
    options: ImportOptions,
) -> Result<bool> {
    validate_row(&row)?;
    if !storage::prediction_exists(conn, &row.prediction_id)? {
        return Err(anyhow!("prediction not found: {}", row.prediction_id));
    }
    let expected_hash = storage::prediction_hash(conn, &row.prediction_id)?;
    let prediction_path = root
        .join("predictions")
        .join(format!("{}.md", row.prediction_id));
    let actual_hash = prediction::prediction_file_hash(&prediction_path).with_context(|| {
        format!(
            "prediction file not readable: {}",
            prediction_path.display()
        )
    })?;
    let contaminated = expected_hash != actual_hash;

    if storage::retro_exists(conn, &row.prediction_id)? {
        if !options.replace_existing {
            return Err(anyhow!(
                "prediction already has a retro: {}",
                row.prediction_id
            ));
        }
        storage::delete_retros_for_prediction(conn, &row.prediction_id)?;
    }

    storage::insert_retro(
        conn,
        &storage::RetroInput {
            prediction_id: row.prediction_id,
            plays: row.plays,
            likes: row.likes,
            comments: row.comments,
            shares: row.shares,
            saves: row.saves,
            top_comments: row.top_comments,
            notes: row.notes,
            contaminated,
        },
    )?;

    Ok(contaminated)
}

fn validate_row(row: &ImportRow) -> Result<()> {
    if row.prediction_id.trim().is_empty() {
        return Err(anyhow!("prediction_id is required"));
    }
    for (name, value) in [
        ("plays", row.plays),
        ("likes", row.likes),
        ("comments", row.comments),
        ("shares", row.shares),
        ("saves", row.saves),
    ] {
        if value < 0 {
            return Err(anyhow!("{name} must be non-negative"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{prediction, score, storage};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn import_with_replace_existing_replaces_completed_sample() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        storage::init_project(root).unwrap();
        let (_paths, conn) = storage::open_project(root).unwrap();
        let prediction_id = create_prediction(root, &conn);

        let first_path = root.join("first.json");
        let second_path = root.join("second.json");
        write_retro_json(&first_path, &prediction_id, 1200);
        write_retro_json(&second_path, &prediction_id, 1800);

        let first = import_file(root, &conn, &first_path).unwrap();
        assert_eq!(first.imported, 1);
        assert_eq!(first.failed, 0);

        let second = import_file_with_options(
            root,
            &conn,
            &second_path,
            ImportOptions {
                replace_existing: true,
            },
        )
        .unwrap();
        assert_eq!(second.imported, 1);
        assert_eq!(second.failed, 0);

        let samples = storage::completed_samples(&conn).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].plays, 1800);
    }

    fn create_prediction(root: &Path, conn: &Connection) -> String {
        fs::create_dir_all(root.join("scripts")).unwrap();
        let script_path = root.join("scripts/replace.md");
        fs::write(&script_path, "普通经验分享，开头一般。").unwrap();

        let rubric = storage::active_rubric(conn).unwrap();
        let scores = score::parse_score_pairs("ER=4,HP=5,QL=3,NA=3,AB=4,SR=2,SAT=1").unwrap();
        let composite = rubric.composite(&scores);
        let draft = prediction::build_prediction(
            root,
            &script_path,
            &rubric,
            &scores,
            composite,
            "sample bet",
            None,
        )
        .unwrap();
        prediction::write_prediction(&draft).unwrap();

        let script_ref = script_path.display().to_string();
        storage::insert_prediction(
            conn,
            &storage::PredictionRecord {
                id: &draft.id,
                script_path: &script_ref,
                script_hash: &draft.script_hash,
                rubric: &rubric,
                scores: &scores,
                composite,
                bet: "sample bet",
                bucket: None,
                prediction_hash: &draft.prediction_hash,
            },
        )
        .unwrap();

        draft.id
    }

    fn write_retro_json(path: &Path, prediction_id: &str, plays: i64) {
        fs::write(
            path,
            format!(
                r#"[
  {{
    "prediction_id": "{prediction_id}",
    "plays": {plays},
    "likes": 120,
    "comments": 18,
    "shares": 7,
    "saves": 11,
    "top_comments": ["评论1", "评论2"],
    "notes": "T+3"
  }}
]"#
            ),
        )
        .unwrap();
    }
}
