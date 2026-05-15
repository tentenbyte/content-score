use crate::dimensions::Dimension;
use crate::rubric::Rubric;
use crate::score::ScoreSet;
use anyhow::Result;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PredictionDraft {
    pub id: String,
    pub markdown: String,
    pub prediction_hash: String,
    pub script_hash: String,
    pub path: PathBuf,
}

pub fn build_prediction(
    project_root: &Path,
    script_path: &Path,
    rubric: &Rubric,
    scores: &ScoreSet,
    composite: f64,
    bet: &str,
    bucket: Option<&str>,
) -> Result<PredictionDraft> {
    let script_bytes = fs::read(script_path)?;
    let script_hash = sha256_hex(&script_bytes);
    let id = prediction_id(script_path, &script_hash);
    let path = project_root.join("predictions").join(format!("{id}.md"));
    let markdown = render_markdown(&RenderContext {
        id: &id,
        script_path,
        script_hash: &script_hash,
        rubric,
        scores,
        composite,
        bet,
        bucket,
    });
    let prediction_hash = sha256_hex(markdown.as_bytes());

    Ok(PredictionDraft {
        id,
        markdown,
        prediction_hash,
        script_hash,
        path,
    })
}

pub fn write_prediction(draft: &PredictionDraft) -> Result<()> {
    if let Some(parent) = draft.path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&draft.path, &draft.markdown)?;
    Ok(())
}

pub fn prediction_file_hash(path: &Path) -> Result<String> {
    Ok(sha256_hex(&fs::read(path)?))
}

fn prediction_id(script_path: &Path, script_hash: &str) -> String {
    let date = Utc::now().date_naive();
    let stem = script_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("script")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("{date}_{}_{}", &script_hash[..12], stem)
}

struct RenderContext<'a> {
    id: &'a str,
    script_path: &'a Path,
    script_hash: &'a str,
    rubric: &'a Rubric,
    scores: &'a ScoreSet,
    composite: f64,
    bet: &'a str,
    bucket: Option<&'a str>,
}

fn render_markdown(ctx: &RenderContext<'_>) -> String {
    let mut lines = Vec::new();
    lines.push(format!("# {} prediction", ctx.id));
    lines.push(String::new());
    lines.push(format!("script_path: {}", ctx.script_path.display()));
    lines.push(format!("script_hash: {}", ctx.script_hash));
    lines.push(format!("rubric_version: {}", ctx.rubric.version));
    lines.push(format!("composite: {:.2}", ctx.composite));
    lines.push(format!("bucket: {}", ctx.bucket.unwrap_or("not-set")));
    lines.push(String::new());
    lines.push("## scores".to_string());
    for dimension in Dimension::all() {
        let entry = ctx
            .scores
            .scores
            .get(dimension)
            .expect("ScoreSet invariant violated: missing dimension");
        lines.push(format!(
            "- {} {}: {} ({})",
            dimension.code(),
            dimension.label(),
            entry.score,
            entry.reason
        ));
    }
    lines.push(String::new());
    lines.push("## bet".to_string());
    lines.push(ctx.bet.to_string());
    lines.push(String::new());
    lines.push("## retro".to_string());
    lines.push("_pending_".to_string());
    lines.push(String::new());
    lines.join("\n")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
