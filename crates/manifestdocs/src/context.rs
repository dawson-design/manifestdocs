//! `manifestdocs context`: a node's or an issue's context for a coding agent, within a token
//! budget.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow, bail};
use manifestdocs_core::check::Snapshot;
use manifestdocs_core::context::{
    Budget, DocsVersion, Request, context, docs_to_read, predecessors_to_read, records_to_read,
};
use manifestdocs_core::paths::RepoPath;
use manifestdocs_core::refs::IssueRef;
use manifestdocs_core::tree::{Entry, Tree};
use manifestdocs_core::work::IssueChain;

use crate::files::{self, ByDefault, Docs, DocsSource};
use crate::locate::locate;
use crate::report::{relative, write_stdout};
use crate::show::{Paths, problems, select_node};
use crate::{gh, git, work};

/// `arg` is a node, or an issue as `owner/name#number`, read from GitHub with `gh`. With
/// `out`, the context goes to that file instead of stdout, and stderr names the file. For an
/// issue, it exits 1 when `gh` can't read one of the issue's parents, which it marks.
pub fn run(
    arg: &str,
    budget: Option<u32>,
    out: Option<&Path>,
    reference: Option<&str>,
) -> Result<ExitCode> {
    let cwd = env::current_dir().context("can't read the current folder")?;
    let docs = files::open(locate(&cwd)?, reference, ByDefault::DefaultBranch, &cwd)?;
    let (tree, snapshot) = files::tree_and_snapshot(&docs, &cwd)?;
    let paths = Paths {
        tree_root: &docs.roots.tree_root,
        cwd: &cwd,
    };
    let chain = IssueRef::parse(arg).map(|issue| issue_chain(&tree, &issue));
    let chain = chain.transpose()?;
    let entry = match &chain {
        Some(chain) => chain.linked_node(&tree)?,
        None => select_node(&tree, &snapshot, arg, &paths)?,
    };
    if let Err(findings) = &entry.node {
        write_stdout(&problems(findings, &paths))?;
        return Ok(ExitCode::from(1));
    }
    let text = assemble(&docs, &tree, snapshot, entry, chain.as_ref(), budget, &cwd);
    write(&text, out, &cwd)?;
    let unread = chain.is_some_and(|chain| chain.parents.iter().any(Result::is_err));
    Ok(match unread {
        true => ExitCode::from(1),
        false => ExitCode::SUCCESS,
    })
}

/// The issue and the issues above it, with which of their sub-issues are closed.
fn issue_chain(tree: &Tree, issue: &IssueRef) -> Result<IssueChain> {
    let Some(github) = &tree.config.github else {
        bail!("`context` reads issues from GitHub, and manifestdocs.yaml has no `github` block");
    };
    let mut read = gh::fetch_chain(&github.host, issue)?.into_iter();
    let Some((reference, first)) = read.next() else {
        bail!("`gh` returned nothing for {issue}");
    };
    let issue = first.map_err(|why| anyhow!("{reference}: {}", work::reason(&why)))?;
    let parents = read.map(|(reference, parent)| parent.map_err(|why| (reference, why)));
    let chain = IssueChain {
        issue,
        parents: parents.collect(),
        closed: BTreeSet::new(),
    };
    let closed = gh::fetch_closed(&github.host, &chain.sub_issues())?;
    Ok(IssueChain { closed, ..chain })
}

/// Reads the docs and decision records the context needs, then assembles it.
fn assemble(
    docs: &Docs,
    tree: &Tree,
    mut snapshot: Snapshot,
    entry: &Entry,
    issue: Option<&IssueChain>,
    budget: Option<u32>,
    cwd: &Path,
) -> String {
    let linked = docs_to_read(tree, &snapshot, entry);
    files::read_linked_docs(&mut snapshot, &linked, docs);
    // Like linked docs, a record that isn't UTF-8 is skipped, and `check` reports it.
    let records = records_to_read(tree, &snapshot, entry);
    files::read_linked_docs(&mut snapshot, &records, docs);
    let predecessors = predecessors_to_read(tree, &snapshot, entry);
    files::read_linked_docs(&mut snapshot, &predecessors, docs);
    let shown = |path: &RepoPath| relative(&docs.roots.repo_root.join(path.as_str()), cwd);
    context(&Request {
        tree,
        snapshot: &snapshot,
        entry,
        issue,
        version: &version(docs, &linked),
        budget: budget.and_then(Budget::new),
        shown: &shown,
    })
}

fn write(text: &str, out: Option<&Path>, cwd: &Path) -> Result<()> {
    let Some(out) = out else {
        return write_stdout(text);
    };
    write_file(&cwd.join(out), text).with_context(|| format!("can't write {}", out.display()))?;
    eprintln!("Wrote {}", out.display());
    Ok(())
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
