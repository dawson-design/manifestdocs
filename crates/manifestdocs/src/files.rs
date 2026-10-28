//! Reads the tree's config, the repo's file list and the tree's Markdown.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use ignore::{DirEntry, WalkBuilder};
use manifestdocs_core::check::{Snapshot, markdown_to_read, outside_anchor_targets};
use manifestdocs_core::config::{self, TreeConfig};
use manifestdocs_core::paths::RepoPath;
use manifestdocs_core::tree::{Tree, read_tree};

use crate::locate::Roots;
use crate::report::relative;

pub fn read_config(tree_root: &Path, cwd: &Path) -> Result<TreeConfig> {
    let path = tree_root.join(config::FILE_NAME);
    let shown = relative(&path, cwd);
    let text = fs::read_to_string(&path).with_context(|| format!("can't read {shown}"))?;
    config::parse(&text).map_err(|error| match error.line {
        Some(line) => anyhow!("{shown}:{line}: {}", error.message),
        None => anyhow!("{shown}: {}", error.message),
    })
}

/// Reads the tree's config and nodes, for the commands that read a tree without checking it.
pub fn tree(roots: &Roots, cwd: &Path) -> Result<Tree> {
    let config = read_config(&roots.tree_root, cwd)?;
    let snapshot = snapshot(roots, &config, cwd)?;
    Ok(read_tree(&config, &snapshot))
}

/// Lists the repo's files and reads the tree's Markdown.
pub fn snapshot(roots: &Roots, config: &TreeConfig, cwd: &Path) -> Result<Snapshot> {
    let tree_root = repo_path(&roots.repo_root, &roots.tree_root)
        .context("the tree's folder path isn't UTF-8")?;
    let paths = list_files(&roots.repo_root)?;
    let mut snapshot = Snapshot {
        tree_root,
        paths,
        markdown: BTreeMap::new(),
    };
    let in_tree = markdown_to_read(config, &snapshot.tree_root, &snapshot.paths);
    read_into(&mut snapshot, in_tree, &roots.repo_root, cwd)?;
    Ok(snapshot)
}

/// Reads the Markdown outside the tree that links point into with an anchor, so `check`
/// can check those anchors too.
pub fn read_anchor_targets(
    snapshot: &mut Snapshot,
    roots: &Roots,
    config: &TreeConfig,
    cwd: &Path,
) -> Result<()> {
    let outside = outside_anchor_targets(config, snapshot);
    read_into(snapshot, outside, &roots.repo_root, cwd)
}

fn read_into(
    snapshot: &mut Snapshot,
    paths: impl IntoIterator<Item = RepoPath>,
    repo_root: &Path,
    cwd: &Path,
) -> Result<()> {
    for path in paths {
        let file = repo_root.join(path.as_str());
        let text = fs::read_to_string(&file)
            .with_context(|| format!("can't read {} as UTF-8 text", relative(&file, cwd)))?;
        snapshot.markdown.insert(path, text);
    }
    Ok(())
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
