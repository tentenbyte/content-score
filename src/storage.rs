use crate::rubric::Rubric;
use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

pub const APP_DIR: &str = ".content-score";
pub const DB_FILE: &str = "content.sqlite";
pub const RUBRIC_FILE: &str = "rubric.toml";

#[derive(Debug, Clone)]
pub struct ProjectPaths {
    pub app_dir: PathBuf,
    pub db_path: PathBuf,
    pub rubric_path: PathBuf,
}

impl ProjectPaths {
    pub fn from_root(root: &Path) -> ProjectPaths {
        let app_dir = root.join(APP_DIR);
        ProjectPaths {
            db_path: app_dir.join(DB_FILE),
            rubric_path: app_dir.join(RUBRIC_FILE),
            app_dir,
        }
    }
}

#[derive(Debug, Serialize)]
struct RubricToml {
    active_version: String,
    weights: std::collections::BTreeMap<String, f64>,
}

pub fn init_project(root: &Path) -> Result<ProjectPaths> {
    let paths = ProjectPaths::from_root(root);
    fs::create_dir_all(&paths.app_dir)?;

    let conn = Connection::open(&paths.db_path)?;
    migrate(&conn)?;
    insert_default_rubric(&conn)?;
    write_default_rubric_file(&paths.rubric_path)?;

    Ok(paths)
}

pub fn open_project(root: &Path) -> Result<(ProjectPaths, Connection)> {
    let paths = ProjectPaths::from_root(root);
    let conn = Connection::open(&paths.db_path)?;
    migrate(&conn)?;
    Ok((paths, conn))
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS rubric_versions (
            version TEXT PRIMARY KEY,
            weights_json TEXT NOT NULL,
            active INTEGER NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS candidates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            score_json TEXT,
            composite REAL,
            created_at TEXT NOT NULL,
            scored_at TEXT
        );

        CREATE TABLE IF NOT EXISTS score_runs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            target_type TEXT NOT NULL,
            target_ref TEXT NOT NULL,
            rubric_version TEXT NOT NULL,
            scores_json TEXT NOT NULL,
            composite REAL NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS predictions (
            id TEXT PRIMARY KEY,
            script_path TEXT NOT NULL,
            script_hash TEXT NOT NULL,
            rubric_version TEXT NOT NULL,
            scores_json TEXT NOT NULL,
            composite REAL NOT NULL,
            bet TEXT NOT NULL,
            bucket TEXT,
            prediction_hash TEXT NOT NULL,
            contaminated INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS retros (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            prediction_id TEXT NOT NULL,
            plays INTEGER NOT NULL,
            likes INTEGER NOT NULL,
            comments INTEGER NOT NULL,
            shares INTEGER NOT NULL,
            saves INTEGER NOT NULL,
            top_comments TEXT,
            notes TEXT,
            contaminated INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY(prediction_id) REFERENCES predictions(id)
        );

        CREATE TABLE IF NOT EXISTS upgrade_proposals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            from_version TEXT NOT NULL,
            to_version TEXT NOT NULL,
            weights_json TEXT NOT NULL,
            rationale TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            applied_at TEXT
        );
        "#,
    )?;
    Ok(())
}

fn insert_default_rubric(conn: &Connection) -> Result<()> {
    let rubric = Rubric::default_v0();
    let weights_json = serde_json::to_string(&rubric.weights_by_code())?;
    conn.execute(
        r#"
        INSERT OR IGNORE INTO rubric_versions (version, weights_json, active, created_at)
        VALUES (?1, ?2, 1, ?3)
        "#,
        params![rubric.version, weights_json, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

fn write_default_rubric_file(path: &Path) -> Result<()> {
    let rubric = Rubric::default_v0();
    let doc = RubricToml {
        active_version: rubric.version.clone(),
        weights: rubric.weights_by_code(),
    };
    fs::write(path, toml::to_string_pretty(&doc)?)?;
    Ok(())
}
