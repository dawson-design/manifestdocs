//! Finds the tree root and the repo root: by walking up from the current folder, then
//! through the nearest `.manifestdocs/config.yaml` pointer.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use manifestdocs_core::config;
use manifestdocs_core::paths::TreePath;
use manifestdocs_core::pointer::{self, Pointer};

use crate::report::relative;

pub struct Roots {
    pub repo_root: PathBuf,
    pub tree_root: PathBuf,
}

pub fn locate(start: &Path) -> Result<Roots> {
    let tree_root = match walk_up(start) {
        Some(tree_root) => tree_root,
        None => from_pointer(start)?,
    };
    Ok(Roots {
        repo_root: repo_root(&tree_root),
        tree_root,
    })
}

/// The nearest folder at or above `start` with a `manifestdocs.yaml`, stopping at the repo root.
fn walk_up(start: &Path) -> Option<PathBuf> {
    for folder in start.ancestors() {
        if folder.join(config::FILE_NAME).is_file() {
            return Some(folder.to_path_buf());
        }
        if is_repo_root(folder) {
            break;
        }
    }
    None
}

/// The tree that the nearest pointer at or above `start` names.
fn from_pointer(start: &Path) -> Result<PathBuf> {
    let Some(folder) = start
        .ancestors()
        .find(|folder| folder.join(pointer::FILE_PATH).is_file())
    else {
        bail!(
            "no {} in this folder or any folder above it, up to the repo root, and no {} at or above it",
            config::FILE_NAME,
            pointer::FILE_PATH
        );
    };
    let path = folder.join(pointer::FILE_PATH);
    let shown = relative(&path, start);
    let tree = read_pointer(&path, &shown)?.tree;
    // Canonical, so a tree path that climbs with `..` prints as a plain path.
    fs::canonicalize(folder.join(tree.as_path()))
        .ok()
        .filter(|tree| tree.join(config::FILE_NAME).is_file())
        .ok_or_else(|| {
            let tree = tree.as_path().display();
            anyhow!("{shown}: tree `{tree}` has no {}", config::FILE_NAME)
        })
}

fn read_pointer(path: &Path, shown: &str) -> Result<Pointer> {
    let text = fs::read_to_string(path).with_context(|| format!("can't read {shown}"))?;
    pointer::parse(&text).map_err(|error| match error.line {
        Some(line) => anyhow!("{shown}:{line}: {}", error.message),
        None => anyhow!("{shown}: {}", error.message),
    })
}

/// `arg`, a path from the current folder to a file or folder that exists, as a path in the tree.
pub fn tree_path(arg: &str, cwd: &Path, tree_root: &Path) -> Result<TreePath> {
    let path = fs::canonicalize(cwd.join(arg)).map_err(|_| anyhow!("no file or folder `{arg}`"))?;
    let root = fs::canonicalize(tree_root).context("can't read the tree's folder")?;
    let inside = path
        .strip_prefix(&root)
        .map_err(|_| anyhow!("`{arg}` is outside the tree"))?;
    inside
        .to_str()
        .and_then(TreePath::new)
        .ok_or_else(|| anyhow!("`{arg}` isn't a UTF-8 path"))
}

// Without a repo, the tree root is the boundary for link checks.
fn repo_root(tree_root: &Path) -> PathBuf {
    tree_root
        .ancestors()
        .find(|folder| is_repo_root(folder))
        .unwrap_or(tree_root)
        .to_path_buf()
}

fn is_repo_root(folder: &Path) -> bool {
    folder.join(".git").exists()
}
