//! `manifestdocs show`: one node in full, with its parent and supporting docs.

use std::env;
use std::fmt::Write as _;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow};
use manifestdocs_core::check::Snapshot;
use manifestdocs_core::finding::Finding;
use manifestdocs_core::id::{NodeId, NodeIdPrefix};
use manifestdocs_core::node::{Node, NodeStatus};
use manifestdocs_core::record::{DecisionIdPrefix, read_records};
use manifestdocs_core::tree::{Entry, NodeSelector, SelectError, Tree};

mod record;

use crate::files::{self, ByDefault};
use crate::json;
use crate::locate::locate;
use crate::report::{finding_line, relative, write_stdout};
use crate::tree::path_in_tree;

pub fn run(arg: &str, json: bool, reference: Option<&str>) -> Result<ExitCode> {
    let cwd = env::current_dir().context("can't read the current folder")?;
    let docs = files::open(locate(&cwd)?, reference, ByDefault::WorkingTree, &cwd)?;
    let roots = &docs.roots;
    let (tree, mut snapshot) = files::tree_and_snapshot(&docs, &cwd)?;
    let paths = Paths {
        tree_root: &roots.tree_root,
        cwd: &cwd,
    };
    let selector = show_selector(&tree, &snapshot, arg, &paths)?;
    if let NodeSelector::Decision(prefix) = &selector {
        files::read_records(&mut snapshot, &docs, &tree.config, &cwd)?;
        let records = read_records(&tree.config, &snapshot);
        return record::run(&tree, &records, arg, prefix, &paths, json);
    }
    let entry = tree
        .select(&selector)
        .map_err(|error| select_error(arg, error, &paths))?;
    if json {
        json::show(&tree, entry, &roots.tree_root, &cwd)?;
        let failed = entry.node.is_err();
        return Ok(if failed {
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        });
    }
    match &entry.node {
        Ok(node) => {
            write_stdout(&render(&tree, entry, node, &paths))?;
            Ok(ExitCode::SUCCESS)
        }
        Err(findings) => {
            write_stdout(&problems(findings, &paths))?;
            Ok(ExitCode::from(1))
        }
    }
}

/// Output paths are shown from the current folder, like the paths `check` prints.
pub struct Paths<'a> {
    pub tree_root: &'a Path,
    pub cwd: &'a Path,
}

impl Paths<'_> {
    fn shown(&self, path: &str) -> String {
        relative(&self.tree_root.join(path), self.cwd)
    }
}

/// The node `arg` names, or an error that says why it names no single node.
pub fn select_node<'a>(
    tree: &'a Tree,
    snapshot: &Snapshot,
    arg: &str,
    paths: &Paths,
) -> Result<&'a Entry> {
    let selector = selector(tree, snapshot, arg, paths)?;
    tree.select(&selector)
        .map_err(|error| select_error(arg, error, paths))
}

// For `show` only, an argument that starts with `dc-` is always a decision ID, so a folder
// with that name is `./dc-x`. Other commands read it as a path.
fn show_selector(
    tree: &Tree,
    snapshot: &Snapshot,
    arg: &str,
    paths: &Paths,
) -> Result<NodeSelector> {
    if !arg.starts_with("dc-") {
        return selector(tree, snapshot, arg, paths);
    }
    let prefix = DecisionIdPrefix::parse(arg).ok_or_else(|| {
        anyhow!("`{arg}` isn't a decision ID: expected `dc-` and 4 to 7 lowercase hex characters")
    })?;
    Ok(NodeSelector::Decision(prefix))
}

// An argument that starts with `fd-` is always an ID, so a folder with that name is `./fd-x`.
fn selector(tree: &Tree, snapshot: &Snapshot, arg: &str, paths: &Paths) -> Result<NodeSelector> {
    if arg.starts_with("fd-") {
        let prefix = NodeIdPrefix::parse(arg).ok_or_else(|| {
            anyhow!("`{arg}` isn't a node ID: expected `fd-` and 4 to 6 lowercase hex characters")
        })?;
        return Ok(NodeSelector::Id(prefix));
    }
    let path = path_in_tree(tree, snapshot, arg, paths.cwd, paths.tree_root)?;
    Ok(NodeSelector::Path(path))
}

fn select_error(arg: &str, error: SelectError, paths: &Paths) -> anyhow::Error {
    match error {
        SelectError::NoMatch => anyhow!("no node `{arg}`"),
        SelectError::NotANode => anyhow!("`{arg}` isn't a node file or node folder"),
        SelectError::Ambiguous(entries) => {
            let mut message = format!("`{arg}` matches {} nodes:", entries.len());
            for entry in entries {
                let file = paths.shown(entry.file.as_str());
                let _ = write!(message, "\n  {}  {file}", entry.id);
            }
            anyhow!(message)
        }
    }
}

fn render(tree: &Tree, entry: &Entry, node: &Node, paths: &Paths) -> String {
    let mut out = header(tree, entry, node, paths);
    let body = node.body.trim_start_matches(['\n', '\r']).trim_end();
    if !body.is_empty() {
        let _ = write!(out, "\n{body}\n");
    }
    let docs = tree.supporting_docs(&entry.file);
    if !docs.is_empty() {
        out.push_str("\nSupporting docs:\n");
        for doc in docs {
            let _ = writeln!(out, "- {}", paths.shown(doc.as_str()));
        }
    }
    out
}

// Lines with nothing to say are left out.
fn header(tree: &Tree, entry: &Entry, node: &Node, paths: &Paths) -> String {
    let mut out = format!("{}\n", tree.heading(entry));
    let _ = writeln!(out, "File: {}", paths.shown(entry.file.as_str()));
    if let Some(parent) = tree.parent(&entry.file) {
        let _ = writeln!(out, "Parent: {} ({})", tree.name(parent), parent.id);
    }
    let _ = writeln!(out, "Summary: {}", node.summary.as_str());
    let repos: Vec<String> = node.repos.iter().map(ToString::to_string).collect();
    let work: Vec<String> = node.work.iter().map(ToString::to_string).collect();
    for (label, items) in [("Repos", repos), ("Work", work)] {
        if !items.is_empty() {
            let _ = writeln!(out, "{label}: {}", items.join(", "));
        }
    }
    if let NodeStatus::Superseded { by } = &node.status {
        let _ = writeln!(out, "Superseded by: {}", replacement(tree, by));
    }
    out
}

/// A node's title and ID, or only the ID when no node has it.
fn replacement(tree: &Tree, id: &NodeId) -> String {
    match tree.entries.values().find(|entry| entry.id == *id) {
        Some(entry) => format!("{} ({id})", tree.name(entry)),
        None => id.to_string(),
    }
}

/// A broken node's errors, in `check`'s format.
pub fn problems(findings: &[Finding], paths: &Paths) -> String {
    let mut findings: Vec<&Finding> = findings.iter().collect();
    findings.sort_by_key(|finding| finding.line);
    let lines = findings.iter().map(|finding| {
        let line = finding_line(finding, paths.tree_root, paths.cwd);
        format!("{line}\n")
    });
    lines.collect()
}
