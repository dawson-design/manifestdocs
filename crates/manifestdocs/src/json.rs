//! `--json` output in schema version 1: one object on stdout holding a command's data, or the
//! error that stopped it. Paths are relative to the current folder, as in text output.

use std::env;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use manifestdocs_core::check::Report;
use manifestdocs_core::config::SUPERSEDED;
use manifestdocs_core::finding::Finding;
use manifestdocs_core::node::{Node, NodeStatus};
use manifestdocs_core::tree::{Entry, Tree};
use serde::Serialize;

use crate::report::{finding_path, relative, write_stdout};

const SCHEMA: u32 = 1;

#[derive(Serialize)]
struct Success<'a, T> {
    schema: u32,
    command: &'a str,
    data: T,
}

#[derive(Serialize)]
struct Failure<'a> {
    schema: u32,
    command: &'a str,
    error: &'a str,
}

#[derive(Serialize)]
struct JsonFinding {
    file: String,
    line: Option<u32>,
    severity: String,
    code: &'static str,
    message: String,
}

#[derive(Serialize)]
struct CheckData {
    nodes: usize,
    findings: Vec<JsonFinding>,
}

#[derive(Serialize)]
struct TreeData<'a> {
    nodes: Vec<TreeNode<'a>>,
}

/// `title` and `status` are `None` when the node's frontmatter has errors.
#[derive(Serialize)]
struct TreeNode<'a> {
    id: &'a str,
    title: Option<&'a str>,
    status: Option<&'a str>,
    file: String,
    parent: Option<&'a str>,
}

/// The fields read from frontmatter are `None` when it has errors, which `findings` lists.
#[derive(Serialize)]
struct ShowData<'a> {
    id: &'a str,
    title: Option<&'a str>,
    status: Option<&'a str>,
    summary: Option<&'a str>,
    file: String,
    parent: Option<&'a str>,
    repos: Option<Vec<String>>,
    work: Option<Vec<String>>,
    superseded_by: Option<&'a str>,
    body: Option<&'a str>,
    supporting_docs: Vec<String>,
    findings: Vec<JsonFinding>,
}

pub fn check(report: &Report, tree_root: &Path, cwd: &Path) -> Result<()> {
    let findings = report.findings.iter();
    let data = CheckData {
        nodes: report.nodes,
        findings: findings.map(|f| finding(f, tree_root, cwd)).collect(),
    };
    print("check", data)
}

/// `lines` is the outline `tree` prints, in order.
pub fn tree(tree: &Tree, lines: &[(usize, &Entry)], tree_root: &Path, cwd: &Path) -> Result<()> {
    let nodes = lines.iter().map(|(_, entry)| {
        let node = entry.node.as_ref().ok();
        TreeNode {
            id: entry.id.as_str(),
            title: node.map(|node| node.title.as_str()),
            status: node.map(status),
            file: shown(entry.file.as_str(), tree_root, cwd),
            parent: tree.parent(&entry.file).map(|parent| parent.id.as_str()),
        }
    });
    print(
        "tree",
        TreeData {
            nodes: nodes.collect(),
        },
    )
}

pub fn show(tree: &Tree, entry: &Entry, tree_root: &Path, cwd: &Path) -> Result<()> {
    let node = entry.node.as_ref().ok();
    let docs = tree.supporting_docs(&entry.file).into_iter();
    let mut findings: Vec<&Finding> = entry.node.as_ref().err().into_iter().flatten().collect();
    findings.sort_by_key(|finding| finding.line);
    let data = ShowData {
        id: entry.id.as_str(),
        title: node.map(|node| node.title.as_str()),
        status: node.map(status),
        summary: node.map(|node| node.summary.as_str()),
        file: shown(entry.file.as_str(), tree_root, cwd),
        parent: tree.parent(&entry.file).map(|parent| parent.id.as_str()),
        repos: node.map(|node| node.repos.iter().map(ToString::to_string).collect()),
        work: node.map(|node| node.work.iter().map(ToString::to_string).collect()),
        superseded_by: node.and_then(superseded_by),
        // Trimmed as text output trims it.
        body: node.map(|node| node.body.trim_start_matches(['\n', '\r']).trim_end()),
        supporting_docs: docs
            .map(|doc| shown(doc.as_str(), tree_root, cwd))
            .collect(),
        findings: findings
            .into_iter()
            .map(|f| finding(f, tree_root, cwd))
            .collect(),
    };
    print("show", data)
}

/// Prints the error that stopped `command`, and exits 2 as text output does.
pub fn fail(command: &str, error: &anyhow::Error) -> ExitCode {
    print_failure(command, &format!("{error:#}"))
}

/// A usage error from clap, as JSON when the arguments ask a command that takes `--json` for
/// it. Anything else, such as help, prints and exits as clap does.
pub fn usage_error(error: clap::Error, cli: &clap::Command) -> ExitCode {
    let args = env::args_os().skip(1);
    let args: Vec<String> = args.map(|arg| arg.to_string_lossy().into_owned()).collect();
    match requested(&args, cli) {
        Some(command) if error.use_stderr() => print_failure(command, &usage_message(&error)),
        _ => error.exit(),
    }
}

/// The subcommand's name, when it takes `--json` and `--json` comes before any `--`. The CLI has
/// no top-level option that takes a value, so the first argument that isn't an option names
/// the subcommand.
fn requested<'a>(args: &[String], cli: &'a clap::Command) -> Option<&'a str> {
    let name = args.iter().find(|arg| !arg.starts_with('-'))?;
    let command = cli.find_subcommand(name)?;
    let mut options = command.get_arguments();
    let takes_json = options.any(|arg| arg.get_long() == Some("json"));
    let mut given = args.iter().take_while(|arg| *arg != "--");
    (takes_json && given.any(|arg| arg == "--json")).then(|| command.get_name())
}

// clap's message without its `error: ` prefix or the usage and help lines after it.
fn usage_message(error: &clap::Error) -> String {
    let text = error.render().to_string();
    let text = text.strip_prefix("error: ").unwrap_or(&text);
    let message = text.split("\n\n").next().unwrap_or(text);
    message.trim_end().to_owned()
}

fn print_failure(command: &str, error: &str) -> ExitCode {
    let failure = Failure {
        schema: SCHEMA,
        command,
        error,
    };
    // A failure that can't be written has nowhere else to go.
    let _ = write(&failure);
    ExitCode::from(2)
}

fn print(command: &str, data: impl Serialize) -> Result<()> {
    write(&Success {
        schema: SCHEMA,
        command,
        data,
    })
}

fn write(value: &impl Serialize) -> Result<()> {
    let mut text = serde_json::to_string(value).context("can't write the JSON output")?;
    text.push('\n');
    write_stdout(&text)
}

fn finding(finding: &Finding, tree_root: &Path, cwd: &Path) -> JsonFinding {
    let problem = &finding.problem;
    JsonFinding {
        file: finding_path(finding, tree_root, cwd),
        line: finding.line,
        severity: problem.severity().to_string(),
        code: problem.code(),
        message: problem.to_string(),
    }
}

fn status(node: &Node) -> &str {
    match &node.status {
        NodeStatus::Active(status) => status.as_str(),
        NodeStatus::Superseded { .. } => SUPERSEDED,
    }
}

fn superseded_by(node: &Node) -> Option<&str> {
    match &node.status {
        NodeStatus::Superseded { by } => Some(by.as_str()),
        NodeStatus::Active(_) => None,
    }
}

fn shown(path: &str, tree_root: &Path, cwd: &Path) -> String {
    relative(&tree_root.join(path), cwd)
}
