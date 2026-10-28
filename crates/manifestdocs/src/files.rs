//! Reads the repo's file list and the tree's Markdown into a snapshot.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use ignore::WalkBuilder;
use manifestdocs_core::check::Snapshot;
use manifestdocs_core::paths::RepoPath;

use crate::locate::Roots;

pub fn snapshot(roots: &Roots) -> Result<Snapshot> {
    let tree_root = repo_path(&roots.repo_root, &roots.tree_root)?;
    let paths = list_files(&roots.repo_root)?;
    let markdown = read_markdown(&roots.repo_root, &paths, &tree_root)?;
    Ok(Snapshot {
        tree_root,
        paths,
        markdown,
    })
}

// GitHub shows dotfiles and ignores a user's global gitignore, so the walk does too.
fn list_files(repo_root: &Path) -> Result<BTreeSet<RepoPath>> {
    let walk = WalkBuilder::new(repo_root)
        .hidden(false)
        .git_global(false)
        .filter_entry(|entry| entry.file_name() != ".git")
        .build();
    let mut paths = BTreeSet::new();
    for entry in walk {
        let entry = entry?;
        if entry.file_type().is_some_and(|kind| kind.is_file()) {
            paths.insert(repo_path(repo_root, entry.path())?);
        }
    }
    Ok(paths)
}

fn read_markdown(
    repo_root: &Path,
    paths: &BTreeSet<RepoPath>,
    tree_root: &RepoPath,
) -> Result<BTreeMap<RepoPath, String>> {
    paths
        .iter()
        .filter(|path| path.as_str().ends_with(".md") && path.in_tree(tree_root).is_some())
        .map(|path| {
            let text = fs::read_to_string(repo_root.join(path.as_str()))
                .with_context(|| format!("can't read {path} as UTF-8 text"))?;
            Ok((path.clone(), text))
        })
        .collect()
}

fn repo_path(repo_root: &Path, path: &Path) -> Result<RepoPath> {
    let relative = path.strip_prefix(repo_root)?;
    let text = relative
        .to_str()
        .with_context(|| format!("{} isn't a UTF-8 path", path.display()))?;
    RepoPath::new(text).with_context(|| format!("{text} is outside the repo"))
}
