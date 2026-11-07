//! The docs clone's branch and commit, read with `git`, for the context header.

use std::path::{Path, PathBuf};
use std::process::Command;

use manifestdocs_core::context::{Changes, DocsVersion};

/// The version of the working tree at `tree_root`. `changes` covers the tree's folder and
/// `docs`, the files outside it that the context loads. Without git, or without a commit,
/// it's `NotInGit`, so a tree still gets its context.
pub fn version(tree_root: &Path, docs: &[PathBuf]) -> DocsVersion {
    let Some(head) = git(tree_root, &["log", "-1", "--format=%h %cs"]) else {
        return DocsVersion::NotInGit;
    };
    let Some((commit, date)) = head.split_once(' ') else {
        return DocsVersion::NotInGit;
    };
    let branch = git(tree_root, &["symbolic-ref", "--quiet", "--short", "HEAD"]);
    let origin_head = git(
        tree_root,
        &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
    );
    DocsVersion::WorkingTree {
        branch,
        commit: commit.to_owned(),
        date: date.to_owned(),
        changes: changes(tree_root, docs),
        default_branch: origin_head
            .and_then(|head| head.strip_prefix("origin/").map(str::to_owned)),
    }
}

/// Whether `git status` lists changes in the tree's folder or `docs`. A failed status is
/// `Unknown`, so the header never claims clean docs it couldn't check.
fn changes(tree_root: &Path, docs: &[PathBuf]) -> Changes {
    let mut status = vec![
        "status".into(),
        "--porcelain".into(),
        "--".into(),
        ".".into(),
    ];
    status.extend(docs.iter().map(|doc| doc.display().to_string()));
    let status: Vec<&str> = status.iter().map(String::as_str).collect();
    match git(tree_root, &status) {
        None => Changes::Unknown,
        Some(changes) if changes.is_empty() => Changes::Clean,
        Some(_) => Changes::Uncommitted,
    }
}

/// `git`'s trimmed output, or `None` when it fails or isn't installed.
fn git(folder: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(args)
        .output()
        .ok()?;
    let stdout = String::from_utf8(output.stdout).ok()?;
    output.status.success().then(|| stdout.trim().to_owned())
}
