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
    Budget, Context as Assembled, DocsVersion, Omitted, Piece, Request, context, docs_to_read,
    predecessors_to_read, records_to_read,
};
use manifestdocs_core::paths::RepoPath;
use manifestdocs_core::refs::IssueRef;
use manifestdocs_core::tree::{Entry, Tree};
use manifestdocs_core::work::IssueChain;
use serde::Serialize;

use crate::files::{self, ByDefault, Docs, DocsSource};
use crate::json;
use crate::locate::locate;
use crate::report::{relative, write_stdout};
use crate::show::{Paths, problems, select_node};
use crate::{gh, git, work};

/// `arg` is a node, or an issue as `owner/name#number`, read from GitHub with `gh`. With
/// `out`, the context goes to that file instead of stdout, and stderr names the file.
pub struct Args<'a> {
    pub arg: &'a str,
    pub budget: Option<u32>,
    pub out: Option<&'a Path>,
    pub reference: Option<&'a str>,
    pub json: bool,
}

/// For an issue, it exits 1 when `gh` can't read one of the issue's parents, which it marks.
pub fn run(args: &Args) -> Result<ExitCode> {
    let cwd = env::current_dir().context("can't read the current folder")?;
    let docs = files::open(
        locate(&cwd)?,
        args.reference,
        ByDefault::DefaultBranch,
        &cwd,
    )?;
    let (tree, snapshot) = files::tree_and_snapshot(&docs, &cwd)?;
    let paths = Paths {
        tree_root: &docs.roots.tree_root,
        cwd: &cwd,
    };
    let chain = IssueRef::parse(args.arg).map(|issue| issue_chain(&tree, &issue));
    let chain = chain.transpose()?;
    let entry = match &chain {
        Some(chain) => chain.linked_node(&tree, &snapshot)?,
        None => select_node(&tree, &snapshot, args.arg, &paths)?,
    };
    if let Err(findings) = &entry.node {
        let problems = problems(findings, &paths);
        match args.json {
            true => json::error("context", problems.trim_end())?,
            false => write_stdout(&problems)?,
        }
        return Ok(ExitCode::from(1));
    }
    let issue = chain.as_ref();
    let (assembled, version) = assemble(&docs, &tree, snapshot, entry, issue, args.budget, &cwd);
    let text = match args.json {
        true => json::text("context", data(&assembled, &version))?,
        false => assembled.markdown,
    };
    write(&text, args.out, &cwd)?;
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

/// Reads the docs and decision records the context needs, then assembles it, with the version
/// it was read from.
fn assemble(
    docs: &Docs,
    tree: &Tree,
    mut snapshot: Snapshot,
    entry: &Entry,
    issue: Option<&IssueChain>,
    budget: Option<u32>,
    cwd: &Path,
) -> (Assembled, DocsVersion) {
    let linked = docs_to_read(tree, &snapshot, entry);
    files::read_linked_docs(&mut snapshot, &linked, docs);
    // Like linked docs, a record that isn't UTF-8 is skipped, and `check` reports it.
    let records = records_to_read(tree, &snapshot, entry);
    files::read_linked_docs(&mut snapshot, &records, docs);
    let predecessors = predecessors_to_read(tree, &snapshot, entry);
    files::read_linked_docs(&mut snapshot, &predecessors, docs);
    let shown = |path: &RepoPath| relative(&docs.roots.repo_root.join(path.as_str()), cwd);
    let version = version(docs, &linked);
    let assembled = context(&Request {
        tree,
        snapshot: &snapshot,
        entry,
        issue,
        version: &version,
        budget: budget.and_then(Budget::new),
        shown: &shown,
    });
    (assembled, version)
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

#[derive(Serialize)]
struct Data<'a> {
    target: &'a str,
    version: Option<Version<'a>>,
    pieces: Vec<JsonPiece<'a>>,
    omitted: Vec<JsonOmitted<'a>>,
    tokens: u32,
    budget: Option<u32>,
}

/// What the header names. `reference` is `None` on a detached HEAD, and `default_branch` is
/// set only for the working tree, when `origin/HEAD` names one.
#[derive(Serialize)]
struct Version<'a> {
    #[serde(rename = "ref")]
    reference: Option<&'a str>,
    commit: &'a str,
    date: &'a str,
    uncommitted: bool,
    default_branch: Option<&'a str>,
}

#[derive(Serialize)]
struct JsonPiece<'a> {
    kind: &'static str,
    heading: &'a str,
    path: Option<&'a str>,
    text: &'a str,
}

#[derive(Serialize)]
struct JsonOmitted<'a> {
    kind: &'static str,
    path: Option<&'a str>,
    tokens: u32,
}

fn data<'a>(assembled: &'a Assembled, version: &'a DocsVersion) -> Data<'a> {
    Data {
        target: &assembled.target,
        version: json_version(version),
        pieces: assembled.pieces.iter().map(json_piece).collect(),
        omitted: assembled.omitted.iter().map(json_omitted).collect(),
        tokens: assembled.tokens,
        budget: assembled.budget,
    }
}

fn json_version(version: &DocsVersion) -> Option<Version<'_>> {
    match version {
        DocsVersion::NotInGit => None,
        DocsVersion::WorkingTree {
            branch,
            commit,
            date,
            uncommitted,
            default_branch,
        } => Some(Version {
            reference: branch.as_deref(),
            commit,
            date,
            uncommitted: *uncommitted,
            default_branch: default_branch.as_deref(),
        }),
        DocsVersion::Commit {
            reference,
            commit,
            date,
        } => Some(Version {
            reference: Some(reference),
            commit,
            date,
            uncommitted: false,
            default_branch: None,
        }),
    }
}

fn json_piece(piece: &Piece) -> JsonPiece<'_> {
    JsonPiece {
        kind: piece.kind.as_str(),
        heading: &piece.heading,
        path: piece.path.as_deref(),
        text: &piece.text,
    }
}

fn json_omitted(omitted: &Omitted) -> JsonOmitted<'_> {
    JsonOmitted {
        kind: omitted.kind.as_str(),
        path: omitted.path.as_deref(),
        tokens: omitted.tokens,
    }
}
