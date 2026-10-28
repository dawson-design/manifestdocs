//! `manifestdocs show`: one node in full, with its parent and supporting docs.

use std::env;
use std::fmt::Write as _;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow};
use manifestdocs_core::finding::Finding;
use manifestdocs_core::id::{NodeId, NodeIdPrefix};
use manifestdocs_core::node::{Node, NodeStatus};
use manifestdocs_core::tree::{Entry, NodeSelector, SelectError, Tree};

use crate::files;
use crate::locate::locate;
use crate::report::{finding_line, relative, write_stdout};
use crate::tree::{heading, name, path_in_tree};

pub fn run(arg: &str) -> Result<ExitCode> {
    let cwd = env::current_dir().context("can't read the current folder")?;
    let roots = locate(&cwd)?;
    let tree = files::tree(&roots, &cwd)?;
    let paths = Paths {
        tree_root: &roots.tree_root,
        cwd: &cwd,
    };
    let selector = selector(&tree, arg, &cwd, &roots.tree_root)?;
    let entry = tree
        .select(&selector)
        .map_err(|error| select_error(arg, error, &paths))?;
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
struct Paths<'a> {
    tree_root: &'a Path,
    cwd: &'a Path,
}

impl Paths<'_> {
    fn shown(&self, path: &str) -> String {
        relative(&self.tree_root.join(path), self.cwd)
    }
}

// An argument that starts with `fd-` is always an ID, so a folder with that name is `./fd-x`.
fn selector(tree: &Tree, arg: &str, cwd: &Path, tree_root: &Path) -> Result<NodeSelector> {
    if arg.starts_with("fd-") {
        let prefix = NodeIdPrefix::parse(arg).ok_or_else(|| {
            anyhow!("`{arg}` isn't a node ID: expected `fd-` and 4 to 6 lowercase hex characters")
        })?;
        return Ok(NodeSelector::Id(prefix));
    }
    Ok(NodeSelector::Path(path_in_tree(tree, arg, cwd, tree_root)?))
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
    let mut out = format!("{}\n", heading(tree, entry));
    let _ = writeln!(out, "File: {}", paths.shown(entry.file.as_str()));
    if let Some(parent) = tree.parent(&entry.file) {
        let _ = writeln!(out, "Parent: {} ({})", name(tree, parent), parent.id);
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

fn replacement(tree: &Tree, id: &NodeId) -> String {
    match tree.entries.values().find(|entry| entry.id == *id) {
        Some(entry) => format!("{} ({id})", name(tree, entry)),
        None => id.to_string(),
    }
}

fn problems(findings: &[Finding], paths: &Paths) -> String {
    let mut findings: Vec<&Finding> = findings.iter().collect();
    findings.sort_by_key(|finding| finding.line);
    let lines = findings.iter().map(|finding| {
        let line = finding_line(finding, paths.tree_root, paths.cwd);
        format!("{line}\n")
    });
    lines.collect()
}
