mod check;
mod files;
mod locate;
mod report;

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
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Check => check::run(),
    };
    result.unwrap_or_else(|error| {
        eprintln!("error: {error:#}");
        ExitCode::from(2)
    })
}
