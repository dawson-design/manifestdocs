//! `manifestdocs context`: a node's context for a coding agent, within a token budget.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use manifestdocs_core::context::{Budget, Request, context, docs_to_read};
use manifestdocs_core::paths::RepoPath;

use crate::files;
use crate::git;
use crate::locate::locate;
use crate::report::{relative, write_stdout};
use crate::show::{Paths, problems, select_node};

pub fn run(arg: &str, budget: Option<u32>) -> Result<ExitCode> {
    let cwd = env::current_dir().context("can't read the current folder")?;
    let roots = locate(&cwd)?;
    let (tree, mut snapshot) = files::tree_and_snapshot(&roots, &cwd)?;
    let paths = Paths {
        tree_root: &roots.tree_root,
        cwd: &cwd,
    };
    let entry = select_node(&tree, arg, &paths)?;
    if let Err(findings) = &entry.node {
        write_stdout(&problems(findings, &paths))?;
        return Ok(ExitCode::from(1));
    }
    let docs = docs_to_read(&tree, &snapshot, entry);
    files::read_linked_docs(&mut snapshot, &docs, &roots);
    let doc_files: Vec<PathBuf> = docs
        .iter()
        .map(|doc| roots.repo_root.join(doc.as_str()))
        .collect();
    let shown = |path: &RepoPath| relative(&roots.repo_root.join(path.as_str()), &cwd);
    let text = context(&Request {
        tree: &tree,
        snapshot: &snapshot,
        entry,
        version: &git::version(&roots.tree_root, &doc_files),
        budget: budget.and_then(Budget::new),
        shown: &shown,
    });
    write_stdout(&text)?;
    Ok(ExitCode::SUCCESS)
}
