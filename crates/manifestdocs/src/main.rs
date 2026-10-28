mod check;
mod files;
mod init;
mod locate;
mod report;
mod show;
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
    /// Print one node in full, with its parent and supporting docs
    Show {
        /// A node ID or ID prefix (`fd-` and 4 to 6 hex characters), or a node file or folder
        node: String,
    },
    /// Start a tree here: write manifestdocs.yaml and a root node
    Init {
        /// The project's title; the tree's name is the title in kebab case [default: this folder's name]
        title: Option<String>,
    },
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Check => check::run(),
        Command::Tree { path, depth } => tree::run(path.as_deref(), depth),
        Command::Show { node } => show::run(&node),
        Command::Init { title } => init::run(title.as_deref()),
    };
    result.unwrap_or_else(|error| {
        eprintln!("error: {error:#}");
        ExitCode::from(2)
    })
}
