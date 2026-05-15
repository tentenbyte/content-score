mod dimensions;
mod rubric;
mod score;
mod storage;

use anyhow::Result;
use clap::{Parser, Subcommand};

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
    }

    Ok(())
}
