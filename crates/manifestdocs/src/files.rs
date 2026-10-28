//! Reads the repo's file list and the tree's Markdown into a snapshot.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use ignore::{DirEntry, WalkBuilder};
use manifestdocs_core::check::{Snapshot, markdown_to_read};
use manifestdocs_core::config::TreeConfig;
use manifestdocs_core::paths::RepoPath;

use crate::locate::Roots;
use crate::report::relative;

pub fn snapshot(roots: &Roots, config: &TreeConfig, cwd: &Path) -> Result<Snapshot> {
    let tree_root = repo_path(&roots.repo_root, &roots.tree_root)
        .context("the tree's folder path isn't UTF-8")?;
    let paths = list_files(&roots.repo_root)?;
    let mut markdown = BTreeMap::new();
    for path in markdown_to_read(config, &tree_root, &paths) {
        let file = roots.repo_root.join(path.as_str());
        let text = fs::read_to_string(&file)
            .with_context(|| format!("can't read {} as UTF-8 text", relative(&file, cwd)))?;
        markdown.insert(path, text);
    }
    Ok(Snapshot {
        tree_root,
        paths,
        markdown,
    })
}

// GitHub shows dotfiles and applies only the repo's own .gitignore files, so the walk does
// the same. A folder with its own .git is a nested repo or submodule, which GitHub shows as
// a pointer rather than files.
fn list_files(repo_root: &Path) -> Result<BTreeSet<RepoPath>> {
    let root = repo_root.to_path_buf();
    let walk = WalkBuilder::new(repo_root)
        .hidden(false)
        .ignore(false)
        .parents(false)
        .git_global(false)
        .git_exclude(false)
        .filter_entry(move |entry| !is_git_folder(entry) && !is_nested_repo(entry, &root))
        .build();
    let mut paths = BTreeSet::new();
    for entry in walk {
        let entry = entry?;
        let is_file = entry.file_type().is_some_and(|kind| kind.is_file());
        // A file name that isn't UTF-8 can't be a node or a link target, so it's skipped.
        paths.extend(
            is_file
                .then(|| repo_path(repo_root, entry.path()))
                .flatten(),
        );
    }
    Ok(paths)
}

fn is_git_folder(entry: &DirEntry) -> bool {
    entry.file_name() == ".git"
}

fn is_nested_repo(entry: &DirEntry, repo_root: &Path) -> bool {
    let is_dir = entry.file_type().is_some_and(|kind| kind.is_dir());
    is_dir && entry.path() != repo_root && entry.path().join(".git").exists()
}

fn repo_path(repo_root: &Path, path: &Path) -> Option<RepoPath> {
    RepoPath::new(path.strip_prefix(repo_root).ok()?.to_str()?)
}
