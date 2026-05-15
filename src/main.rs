mod calibration;
mod dimensions;
mod prediction;
mod retro_import;
mod rubric;
mod score;
mod storage;
mod upgrade;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "content-score")]
#[command(about = "Local content scoring and calibration CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init,
    Score {
        script: PathBuf,
        #[arg(long)]
        scores: Option<String>,
        #[arg(long)]
        score_json: Option<PathBuf>,
        #[arg(long)]
        llm: bool,
    },
    Predict {
        script: PathBuf,
        #[arg(long)]
        scores: Option<String>,
        #[arg(long)]
        score_json: Option<PathBuf>,
        #[arg(long)]
        llm: bool,
        #[arg(long)]
        bet: String,
        #[arg(long)]
        bucket: Option<String>,
    },
    Retro {
        id: String,
        import_file: Option<PathBuf>,
        #[arg(long)]
        plays: Option<i64>,
        #[arg(long)]
        likes: Option<i64>,
        #[arg(long)]
        comments: Option<i64>,
        #[arg(long)]
        shares: Option<i64>,
        #[arg(long)]
        saves: Option<i64>,
        #[arg(long)]
        top_comments: Option<String>,
        #[arg(long)]
        notes: Option<String>,
    },
    Calibrate,
    Upgrade {
        #[arg(long)]
        propose: bool,
        #[arg(long)]
        apply: Option<i64>,
    },
    Candidates {
        #[command(subcommand)]
        command: CandidateCommand,
    },
}

#[derive(Debug, Subcommand)]
enum CandidateCommand {
    Add {
        text: String,
    },
    Score {
        id: i64,
        #[arg(long)]
        scores: Option<String>,
        #[arg(long)]
        score_json: Option<PathBuf>,
        #[arg(long)]
        llm: bool,
    },
    Top,
}

#[cfg(test)]
mod tests {
    #[test]
    fn binary_name_is_content_score() {
        assert_eq!(env!("CARGO_PKG_NAME"), "content-score");
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            storage::init_project(&std::env::current_dir()?)?;
            println!("content-score initialized at .content-score");
            println!("active rubric: v0");
        }
        Commands::Score {
            script,
            scores,
            score_json,
            llm,
        } => {
            let target_text = std::fs::read_to_string(&script)?;
            let scores = resolve_scores(&target_text, scores, score_json, llm)?;
            let (_paths, conn) = storage::open_project(&std::env::current_dir()?)?;
            let rubric = storage::active_rubric(&conn)?;
            let composite = rubric.composite(&scores);
            storage::insert_score_run(
                &conn,
                "script",
                &script.display().to_string(),
                &rubric,
                &scores,
                composite,
            )?;
            print_score_table(&scores, composite);
        }
        Commands::Predict {
            script,
            scores,
            score_json,
            llm,
            bet,
            bucket,
        } => {
            let target_text = std::fs::read_to_string(&script)?;
            let scores = resolve_scores(&target_text, scores, score_json, llm)?;
            let root = std::env::current_dir()?;
            let (_paths, conn) = storage::open_project(&root)?;
            let rubric = storage::active_rubric(&conn)?;
            let composite = rubric.composite(&scores);
            let draft = prediction::build_prediction(
                &root,
                &script,
                &rubric,
                &scores,
                composite,
                &bet,
                bucket.as_deref(),
            )?;
            prediction::write_prediction(&draft)?;
            let script_ref = script.display().to_string();
            storage::insert_prediction(
                &conn,
                &storage::PredictionRecord {
                    id: &draft.id,
                    script_path: &script_ref,
                    script_hash: &draft.script_hash,
                    rubric: &rubric,
                    scores: &scores,
                    composite,
                    bet: &bet,
                    bucket: bucket.as_deref(),
                    prediction_hash: &draft.prediction_hash,
                },
            )?;
            println!(
                "prediction {} written to {}",
                draft.id,
                draft.path.display()
            );
            println!("composite: {:.2} / 10", composite);
        }
        Commands::Retro {
            id,
            import_file,
            plays,
            likes,
            comments,
            shares,
            saves,
            top_comments,
            notes,
        } => {
            let root = std::env::current_dir()?;
            let (_paths, conn) = storage::open_project(&root)?;
            if id == "import" {
                let Some(path) = import_file else {
                    anyhow::bail!("retro import requires a CSV or JSON path");
                };
                let summary = retro_import::import_file(&root, &conn, &path)?;
                for failure in &summary.failures {
                    println!(
                        "failed row {} {}: {}",
                        failure.row_number, failure.prediction_id, failure.error
                    );
                }
                println!("imported: {}", summary.imported);
                println!("failed: {}", summary.failed);
                println!("contaminated: {}", summary.contaminated);
            } else {
                if import_file.is_some() {
                    anyhow::bail!("unexpected extra argument after prediction id");
                }
                let plays = require_metric("plays", plays)?;
                let likes = require_metric("likes", likes)?;
                let comments = require_metric("comments", comments)?;
                let shares = require_metric("shares", shares)?;
                let saves = require_metric("saves", saves)?;
                let contaminated = record_retro(
                    &root,
                    &conn,
                    storage::RetroInput {
                        prediction_id: id.clone(),
                        plays,
                        likes,
                        comments,
                        shares,
                        saves,
                        top_comments,
                        notes,
                        contaminated: false,
                    },
                )?;
                if contaminated {
                    eprintln!("integrity warning: prediction file changed since it was written");
                }
                println!("retro recorded for {id}");
            }
        }
        Commands::Calibrate => {
            let (_paths, conn) = storage::open_project(&std::env::current_dir()?)?;
            let samples = storage::completed_samples(&conn)?;
            let report = calibration::analyze(&samples);
            println!("{}", report.render());
        }
        Commands::Upgrade { propose, apply } => {
            let (_paths, conn) = storage::open_project(&std::env::current_dir()?)?;
            match (propose, apply) {
                (true, None) => {
                    let samples = storage::completed_samples(&conn)?;
                    let report = calibration::analyze(&samples);
                    let rubric = storage::active_rubric(&conn)?;
                    let to_version = upgrade::next_version(&rubric.version);
                    let weights = calibration::propose_weights(&rubric, &report);
                    let rationale = report.render();
                    let id = storage::insert_upgrade_proposal(
                        &conn,
                        &rubric.version,
                        &to_version,
                        &weights,
                        &rationale,
                    )?;
                    println!(
                        "upgrade proposal #{id}: {} -> {}",
                        rubric.version, to_version
                    );
                    println!("{rationale}");
                }
                (false, Some(id)) => {
                    let version = storage::apply_upgrade_proposal(&conn, id)?;
                    println!("active rubric: {version}");
                }
                _ => anyhow::bail!("use either upgrade --propose or upgrade --apply <id>"),
            }
        }
        Commands::Candidates { command } => {
            let (_paths, conn) = storage::open_project(&std::env::current_dir()?)?;
            let rubric = storage::active_rubric(&conn)?;
            match command {
                CandidateCommand::Add { text } => {
                    let id = storage::add_candidate(&conn, &text)?;
                    println!("candidate #{id}: {text}");
                }
                CandidateCommand::Score {
                    id,
                    scores,
                    score_json,
                    llm,
                } => {
                    let target_text = if llm {
                        storage::candidate_text(&conn, id)?
                    } else {
                        String::new()
                    };
                    let scores = resolve_scores(&target_text, scores, score_json, llm)?;
                    let composite = rubric.composite(&scores);
                    storage::score_candidate(&conn, id, &scores, composite)?;
                    storage::insert_score_run(
                        &conn,
                        "candidate",
                        &id.to_string(),
                        &rubric,
                        &scores,
                        composite,
                    )?;
                    println!("candidate #{id} candidate_score: {:.2} / 10", composite);
                }
                CandidateCommand::Top => {
                    for candidate in storage::list_candidates(&conn)? {
                        match candidate.composite {
                            Some(score) => println!(
                                "#{} candidate_score: {:.2} / 10  {}",
                                candidate.id, score, candidate.text
                            ),
                            None => {
                                println!(
                                    "#{} candidate_score: unscored  {}",
                                    candidate.id, candidate.text
                                )
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn print_score_table(scores: &score::ScoreSet, composite: f64) {
    for dimension in dimensions::Dimension::all() {
        let entry = scores
            .scores
            .get(dimension)
            .expect("ScoreSet invariant violated: missing dimension");
        println!(
            "{}  {}  {}",
            dimension.code(),
            entry.score,
            dimension.label()
        );
    }
    println!("composite: {:.2} / 10", composite);
}

fn resolve_scores(
    target_text: &str,
    scores: Option<String>,
    score_json: Option<PathBuf>,
    llm: bool,
) -> Result<score::ScoreSet> {
    let selected = scores.is_some() as u8 + score_json.is_some() as u8 + llm as u8;
    if selected != 1 {
        anyhow::bail!("choose exactly one scoring input: --scores, --score-json, or --llm");
    }

    if let Some(scores) = scores {
        return score::parse_score_pairs(&scores);
    }
    if let Some(path) = score_json {
        return score::load_score_json(&path);
    }
    score::score_with_llm(target_text)
}

fn require_metric(name: &str, value: Option<i64>) -> Result<i64> {
    value.ok_or_else(|| anyhow::anyhow!("--{name} is required"))
}

fn record_retro(
    root: &std::path::Path,
    conn: &rusqlite::Connection,
    mut input: storage::RetroInput,
) -> Result<bool> {
    let expected_hash = storage::prediction_hash(conn, &input.prediction_id)?;
    let prediction_path = root
        .join("predictions")
        .join(format!("{}.md", input.prediction_id));
    let actual_hash = prediction::prediction_file_hash(&prediction_path)?;
    let contaminated = expected_hash != actual_hash;
    input.contaminated = contaminated;
    storage::insert_retro(conn, &input)?;
    Ok(contaminated)
}
