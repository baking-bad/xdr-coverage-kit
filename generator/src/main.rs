//! `generate` writes the corpus, and `check` verifies it.

mod check;
mod codec;
mod contract;
mod files;
mod generate;
mod manifest;
mod network;
mod node;
mod scenarios;

use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};
use node::NodeConfig;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Generates and checks a corpus of rare Stellar XDR shapes")]
struct Cli {
    /// The directory of the corpus.
    #[arg(long, default_value = "corpus", global = true)]
    corpus: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run every scenario, write the corpus, and check it.
    Generate {
        /// The stellar-core 30.0.0 program: a name on PATH or a path.
        #[arg(long, env = "STELLAR_CORE", default_value = "stellar-core")]
        stellar_core: PathBuf,
    },
    /// Verify the schema file and every source and entry of the corpus.
    Check,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    ensure!(
        cli.corpus.is_dir(),
        "the corpus directory {} does not exist: run from the root of the \
         repository, or name the directory with --corpus",
        cli.corpus.display()
    );
    match cli.command {
        Command::Generate { stellar_core } => {
            generate::run(&cli.corpus, &NodeConfig::standalone(stellar_core))
        }
        Command::Check => check::run(&cli.corpus),
    }
}
