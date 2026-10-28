mod check;
mod files;
mod locate;
mod report;
mod tree;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Feature trees, agent context and decision records for docs repos.
#[derive(Parser)]
#[command(name = "manifestdocs", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate the tree, exiting with 1 when it has errors
    Check,
    /// Print an outline of the nodes, with their IDs, statuses and files
    Tree {
        /// A node file, or a folder whose nodes to outline
        path: Option<String>,
        /// How many levels below the start to show
        #[arg(long)]
        depth: Option<usize>,
    },
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Check => check::run(),
        Command::Tree { path, depth } => tree::run(path.as_deref(), depth),
    };
    result.unwrap_or_else(|error| {
        eprintln!("error: {error:#}");
        ExitCode::from(2)
    })
}
