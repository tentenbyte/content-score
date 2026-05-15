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
        match import_row(root, conn, row) {
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

fn import_row(root: &Path, conn: &Connection, row: ImportRow) -> Result<bool> {
    validate_row(&row)?;
    let expected_hash = storage::prediction_hash(conn, &row.prediction_id)
        .with_context(|| format!("prediction not found: {}", row.prediction_id))?;
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
