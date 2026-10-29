//! `manifestdocs context`: a node's context for a coding agent, within a token budget.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use manifestdocs_core::context::{Budget, DocsVersion, Request, context, docs_to_read};
use manifestdocs_core::paths::RepoPath;

use crate::files::{self, ByDefault, Docs, DocsSource};
use crate::git;
use crate::locate::locate;
use crate::report::{relative, write_stdout};
use crate::show::{Paths, problems, select_node};

/// With `out`, the context goes to that file instead of stdout, and stderr names the file.
pub fn run(
    arg: &str,
    budget: Option<u32>,
    out: Option<&Path>,
    reference: Option<&str>,
) -> Result<ExitCode> {
    let cwd = env::current_dir().context("can't read the current folder")?;
    let docs = files::open(locate(&cwd)?, reference, ByDefault::DefaultBranch, &cwd)?;
    let roots = &docs.roots;
    let (tree, mut snapshot) = files::tree_and_snapshot(&docs, &cwd)?;
    let paths = Paths {
        tree_root: &roots.tree_root,
        cwd: &cwd,
    };
    let entry = select_node(&tree, &snapshot, arg, &paths)?;
    if let Err(findings) = &entry.node {
        write_stdout(&problems(findings, &paths))?;
        return Ok(ExitCode::from(1));
    }
    let linked = docs_to_read(&tree, &snapshot, entry);
    files::read_linked_docs(&mut snapshot, &linked, &docs);
    let shown = |path: &RepoPath| relative(&roots.repo_root.join(path.as_str()), &cwd);
    let text = context(&Request {
        tree: &tree,
        snapshot: &snapshot,
        entry,
        version: &version(&docs, &linked),
        budget: budget.and_then(Budget::new),
        shown: &shown,
    });
    match out {
        Some(out) => {
            write_file(&cwd.join(out), &text)
                .with_context(|| format!("can't write {}", out.display()))?;
            eprintln!("Wrote {}", out.display());
        }
        None => write_stdout(&text)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// Writes `text` to `file`, creating its folders, such as a factory's `.context/`.
fn write_file(file: &Path, text: &str) -> std::io::Result<()> {
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder)?;
    }
    fs::write(file, text)
}

/// The version the context was read from, for its header.
fn version(docs: &Docs, linked: &[RepoPath]) -> DocsVersion {
    match &docs.source {
        DocsSource::WorkingTree => {
            let root = &docs.roots.repo_root;
            let files: Vec<PathBuf> = linked.iter().map(|doc| root.join(doc.as_str())).collect();
            git::version(&docs.roots.tree_root, &files)
        }
        DocsSource::Commit {
            reference,
            commit,
            date,
        } => DocsVersion::Commit {
            reference: reference.clone(),
            commit: commit.clone(),
            date: date.clone(),
        },
    }
}
