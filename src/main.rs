mod dimensions;
mod rubric;
mod score;
mod storage;

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
        scores: String,
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
        scores: String,
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
        Commands::Score { script, scores } => {
            std::fs::read_to_string(&script)?;
            let scores = score::parse_score_pairs(&scores)?;
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
        Commands::Candidates { command } => {
            let (_paths, conn) = storage::open_project(&std::env::current_dir()?)?;
            let rubric = storage::active_rubric(&conn)?;
            match command {
                CandidateCommand::Add { text } => {
                    let id = storage::add_candidate(&conn, &text)?;
                    println!("candidate #{id}: {text}");
                }
                CandidateCommand::Score { id, scores } => {
                    let scores = score::parse_score_pairs(&scores)?;
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
                                println!("#{} candidate_score: unscored  {}", candidate.id, candidate.text)
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
