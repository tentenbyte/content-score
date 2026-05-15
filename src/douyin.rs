use crate::{retro_import, storage};
use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Subcommand)]
pub enum DouyinCommand {
    Doctor,
    Login,
    Fetch {
        prediction_id: String,
        input: String,
        #[arg(long)]
        no_import: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        replace: bool,
    },
}

pub fn handle(command: DouyinCommand) -> Result<()> {
    match command {
        DouyinCommand::Doctor => {
            anyhow::bail!("adapter checks are not implemented yet");
        }
        DouyinCommand::Login => {
            anyhow::bail!("adapter login is not implemented yet");
        }
        DouyinCommand::Fetch {
            prediction_id,
            input,
            no_import,
            dry_run,
            replace,
        } => {
            if dry_run {
                anyhow::bail!("--dry-run is not implemented yet");
            }
            if replace {
                anyhow::bail!("--replace is not implemented yet");
            }
            fetch(prediction_id, input, no_import)
        }
    }
}

fn fetch(prediction_id: String, input: String, no_import: bool) -> Result<()> {
    let adapter_input = resolve_aweme_id(&input)?;
    let root = std::env::current_dir()?;
    let paths = storage::ProjectPaths::from_root(&root);
    if !paths.db_path.exists() {
        anyhow::bail!(
            "content project database not found: {}",
            paths.db_path.display()
        );
    }
    let (_paths, conn) = storage::open_project(&root)?;
    if !storage::prediction_exists(&conn, &prediction_id)? {
        anyhow::bail!("prediction not found: {prediction_id}");
    }

    let adapter_path = adapter_path()?;
    let python = python_command(&root);
    let imports_dir = paths.app_dir.join("imports");
    std::fs::create_dir_all(&imports_dir)?;
    let output_path = imports_dir.join(format!("douyin-{prediction_id}.json"));

    let output = Command::new(&python)
        .arg(&adapter_path)
        .arg("fetch")
        .arg(&adapter_input)
        .arg("--prediction-id")
        .arg(&prediction_id)
        .arg("--output")
        .arg(&output_path)
        .output()
        .with_context(|| {
            format!(
                "failed to launch Douyin adapter with {} {}",
                python.display(),
                adapter_path.display()
            )
        })?;

    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        anyhow::bail!("Douyin adapter failed with status {}", output.status);
    }

    println!("json: {}", output_path.display());
    validate_adapter_output(&output_path, &prediction_id)?;
    if no_import {
        println!("imported: no");
        return Ok(());
    }

    let summary = retro_import::import_file_with_options(
        &root,
        &conn,
        &output_path,
        retro_import::ImportOptions::default(),
    )?;
    for failure in &summary.failures {
        println!(
            "failed row {} {}: {}",
            failure.row_number, failure.prediction_id, failure.error
        );
    }
    println!("imported: {}", summary.imported);
    println!("failed: {}", summary.failed);
    println!("contaminated: {}", summary.contaminated);
    if summary.imported > 0 && summary.failed == 0 {
        println!("imported: yes");
        Ok(())
    } else {
        println!("imported: no");
        anyhow::bail!("Douyin import failed")
    }
}

fn validate_adapter_output(path: &Path, prediction_id: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read Douyin adapter output {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse Douyin adapter output {}", path.display()))?;
    let rows = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("adapter output must be a JSON array"))?;
    if rows.len() != 1 {
        anyhow::bail!(
            "adapter output must contain exactly one row for prediction {prediction_id}, found {}",
            rows.len()
        );
    }

    let row = rows[0]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("adapter output row must be a JSON object"))?;
    match row.get("prediction_id").and_then(|value| value.as_str()) {
        Some(actual) if actual == prediction_id => Ok(()),
        Some(actual) => anyhow::bail!(
            "adapter output prediction_id mismatch: expected {prediction_id}, got {actual}"
        ),
        None => anyhow::bail!("adapter output prediction_id is missing or not a string"),
    }
}

fn adapter_path() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("CONTENT_SCORE_DOUYIN_ADAPTER") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
        anyhow::bail!(
            "Douyin adapter path from CONTENT_SCORE_DOUYIN_ADAPTER does not exist: {}",
            path.display()
        );
    }

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("adapters/douyin-session/cli.py");
    if path.exists() {
        Ok(path)
    } else {
        anyhow::bail!("Douyin adapter path does not exist: {}", path.display())
    }
}

fn python_command(root: &Path) -> PathBuf {
    let venv_python = root.join(".venv/bin/python");
    if venv_python.exists() {
        venv_python
    } else {
        PathBuf::from("python3")
    }
}

pub fn resolve_aweme_id(input: &str) -> Result<String> {
    let input = input.trim();
    if !input.is_empty() && input.chars().all(|c| c.is_ascii_digit()) {
        return Ok(input.to_string());
    }

    if let Some((host, path)) = parse_http_url(input) {
        if host == "v.douyin.com" {
            return Ok(input.to_string());
        }

        if matches!(host.as_str(), "douyin.com" | "www.douyin.com") {
            if let Some(aweme_id) = video_id_from_path(path) {
                return Ok(aweme_id.to_string());
            }
        }
    }

    anyhow::bail!(
        "unsupported Douyin input: expected raw aweme id, douyin.com/video/<id>, or v.douyin.com short link"
    )
}

fn parse_http_url(input: &str) -> Option<(String, &str)> {
    let rest = input
        .strip_prefix("https://")
        .or_else(|| input.strip_prefix("http://"))?;
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = host
        .split_once(':')
        .map_or(host, |(host_without_port, _)| host_without_port)
        .to_ascii_lowercase();

    Some((host, path))
}

fn video_id_from_path(path: &str) -> Option<&str> {
    let mut segments = path.split('/');
    while let Some(segment) = segments.next() {
        if segment == "video" {
            let aweme_id = segments
                .next()?
                .split(['?', '#'])
                .next()
                .unwrap_or_default();
            if !aweme_id.is_empty() && aweme_id.chars().all(|c| c.is_ascii_digit()) {
                return Some(aweme_id);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_raw_and_long_douyin_inputs() {
        assert_eq!(
            resolve_aweme_id("7333333333333333333").unwrap(),
            "7333333333333333333"
        );
        assert_eq!(
            resolve_aweme_id("https://www.douyin.com/video/7333333333333333333").unwrap(),
            "7333333333333333333"
        );
        assert_eq!(
            resolve_aweme_id("https://douyin.com/video/7333333333333333333").unwrap(),
            "7333333333333333333"
        );
    }

    #[test]
    fn accepts_v_douyin_short_link_for_adapter_resolution() {
        assert_eq!(
            resolve_aweme_id("https://v.douyin.com/iF8abc1/").unwrap(),
            "https://v.douyin.com/iF8abc1/"
        );
    }

    #[test]
    fn rejects_invalid_douyin_input() {
        let error = resolve_aweme_id("https://example.com/video/7333333333333333333").unwrap_err();

        assert!(error.to_string().contains("Douyin"));
    }

    #[test]
    fn stubs_return_not_implemented_errors() {
        let doctor_error = handle(DouyinCommand::Doctor).unwrap_err();
        assert!(doctor_error
            .to_string()
            .contains("adapter checks are not implemented yet"));

        let login_error = handle(DouyinCommand::Login).unwrap_err();
        assert!(login_error
            .to_string()
            .contains("adapter login is not implemented yet"));
    }
}
