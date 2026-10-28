//! Finds the tree root and the repo root by walking up from the current folder.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use manifestdocs_core::config;

pub struct Roots {
    pub repo_root: PathBuf,
    pub tree_root: PathBuf,
}

/// Walks up from `start` to the nearest folder with a `manifestdocs.yaml`, stopping at the repo root.
pub fn locate(start: &Path) -> Result<Roots> {
    for folder in start.ancestors() {
        if folder.join(config::FILE_NAME).is_file() {
            return Ok(Roots {
                repo_root: repo_root(folder),
                tree_root: folder.to_path_buf(),
            });
        }
        if is_repo_root(folder) {
            break;
        }
    }
    bail!(
        "no {} in this folder or any folder above it, up to the repo root",
        config::FILE_NAME
    )
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
