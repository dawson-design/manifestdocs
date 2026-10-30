mod check;
mod context;
mod decide;
mod decisions;
mod files;
mod gh;
mod git;
mod init;
mod json;
mod locate;
mod report;
mod show;
mod tree;
mod work;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};

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
    Check {
        /// A file or folder: check the tree that contains it [default: the current folder]
        path: Option<String>,
        /// Exit with 1 on warnings too, such as to gate CI on them
        #[arg(long)]
        strict: bool,
        /// Print JSON, in schema version 1, instead of text
        #[arg(long)]
        json: bool,
        /// Skip the GitHub rules without calling `gh`
        #[arg(long)]
        offline: bool,
        #[command(flatten)]
        docs: DocsRef,
    },
    /// Print an outline of the nodes, with their IDs, statuses and files
    Tree {
        /// A node file, or a folder whose nodes to outline
        path: Option<String>,
        /// How many levels below the start to show
        #[arg(long)]
        depth: Option<usize>,
        /// Print JSON, in schema version 1, instead of text
        #[arg(long)]
        json: bool,
        #[command(flatten)]
        docs: DocsRef,
    },
    /// Print one node in full, with its parent and supporting docs
    Show {
        /// A node ID or ID prefix (`fd-` and 4 to 6 hex characters), or a node file or folder
        node: String,
        /// Print JSON, in schema version 1, instead of text
        #[arg(long)]
        json: bool,
        #[command(flatten)]
        docs: DocsRef,
    },
    /// Print a node's context for a coding agent: the node, the root node, the docs it links
    /// to and the nodes around it. For a GitHub issue, also the issue and its parents, read
    /// with `gh`
    Context {
        /// A node ID or ID prefix (`fd-` and 4 to 6 hex characters), a node file or folder, or
        /// an issue as `owner/name#number`
        node: String,
        /// The output format: Markdown, for agents
        #[arg(long, value_enum, default_value_t = Format::Md)]
        format: Format,
        /// An approximate token budget; what doesn't fit is left out and listed by path
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
        budget: Option<u32>,
        /// Write the context to this file instead of stdout
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
        /// Print JSON, in schema version 1, instead of Markdown
        #[arg(long, conflicts_with = "format")]
        json: bool,
        #[command(flatten)]
        docs: DocsRef,
    },
    /// Print the issues a node lists in `work`, with their sub-issues, read from GitHub with `gh`
    Work {
        /// A node ID or ID prefix (`fd-` and 4 to 6 hex characters), or a node file or folder
        node: String,
        /// Print JSON, in schema version 1, instead of text
        #[arg(long)]
        json: bool,
        #[command(flatten)]
        docs: DocsRef,
    },
    /// Start a tree here: write manifestdocs.yaml and a root node
    Init {
        /// The project's title; the tree's name is the title in kebab case [default: this folder's name]
        title: Option<String>,
    },
    /// Write a decision record from a file or stdin, filling in its ID, date and status, and
    /// print its path
    Decide {
        /// The record in the on-disk format, without `id`, or `-` to read it from stdin
        #[arg(long)]
        file: String,
        /// Print the record and its path without writing it
        #[arg(long)]
        dry_run: bool,
        /// A factory run: write `proposed`, to .manifestdocs/decisions/ in the current repo
        #[arg(long)]
        unattended: bool,
        /// The folder to write to [default: the node's decisions/ folder]
        #[arg(long)]
        out: Option<String>,
        /// Print JSON, in schema version 1, instead of text
        #[arg(long)]
        json: bool,
    },
    /// List decision records for a node and its ancestors, or for the whole tree
    Decisions {
        /// A node ID or ID prefix (`fd-` and 4 to 6 hex characters), or a node file or folder
        node: Option<String>,
        /// Print JSON, in schema version 1, instead of text
        #[arg(long)]
        json: bool,
        #[command(flatten)]
        docs: DocsRef,
    },
}

impl Command {
    /// The command's name, when it's asked for JSON output.
    fn json(&self) -> Option<&'static str> {
        match self {
            Self::Check { json: true, .. } => Some("check"),
            Self::Tree { json: true, .. } => Some("tree"),
            Self::Show { json: true, .. } => Some("show"),
            Self::Decide { json: true, .. } => Some("decide"),
            Self::Decisions { json: true, .. } => Some("decisions"),
            Self::Work { json: true, .. } => Some("work"),
            Self::Context { json: true, .. } => Some("context"),
            _ => None,
        }
    }
}

/// Which version of the docs a read command reads.
#[derive(Args)]
struct DocsRef {
    /// Read the docs at this branch, tag or commit in the docs clone [default: the working
    /// tree inside the docs clone, `origin/HEAD` from another repo]
    #[arg(long = "ref", value_name = "REF")]
    reference: Option<String>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Md,
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => return json::usage_error(error, &Cli::command()),
    };
    let json = cli.command.json();
    let result = match cli.command {
        Command::Check {
            path,
            strict,
            json,
            offline,
            docs,
        } => {
            let github = match offline {
                true => check::GitHub::Skip,
                false => check::GitHub::Check,
            };
            check::run(
                path.as_deref(),
                strict,
                json,
                github,
                docs.reference.as_deref(),
            )
        }
        Command::Tree {
            path,
            depth,
            json,
            docs,
        } => tree::run(path.as_deref(), depth, json, docs.reference.as_deref()),
        Command::Show { node, json, docs } => show::run(&node, json, docs.reference.as_deref()),
        Command::Context {
            node,
            format: Format::Md,
            budget,
            out,
            json,
            docs,
        } => context::run(&context::Args {
            arg: &node,
            budget,
            out: out.as_deref(),
            reference: docs.reference.as_deref(),
            json,
        }),
        Command::Work { node, json, docs } => work::run(&node, json, docs.reference.as_deref()),
        Command::Init { title } => init::run(title.as_deref()),
        Command::Decide {
            file,
            dry_run,
            unattended,
            out,
            json,
        } => decide::run(&decide::Args {
            file: &file,
            dry_run,
            unattended,
            out: out.as_deref(),
            json,
        }),
        Command::Decisions { node, json, docs } => {
            decisions::run(node.as_deref(), json, docs.reference.as_deref())
        }
    };
    result.unwrap_or_else(|error| match json {
        Some(command) => json::fail(command, &error),
        None => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    })
}
