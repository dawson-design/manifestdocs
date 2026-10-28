//! Finds the tree root and the repo root: by walking up from the current folder, then
//! through the workspaces in the user config.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use manifestdocs_core::config;
use manifestdocs_core::workspace::{self, AbsPath, Workspace, workspace_for};

pub struct Roots {
    pub repo_root: PathBuf,
    pub tree_root: PathBuf,
}

/// The user config as messages name it.
const USER_CONFIG: &str = "~/.config/manifestdocs/config.yaml";

pub fn locate(start: &Path) -> Result<Roots> {
    let tree_root = match walk_up(start) {
        Some(tree_root) => tree_root,
        None => from_workspace(start)?,
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

fn from_workspace(start: &Path) -> Result<PathBuf> {
    let not_found = format!(
        "no {} in this folder or any folder above it, up to the repo root",
        config::FILE_NAME
    );
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        bail!(not_found);
    };
    let Some(workspaces) = read_workspaces(&home)? else {
        bail!(not_found);
    };
    // Canonical paths, so a symlinked folder such as macOS's /var still matches.
    let start = fs::canonicalize(start).unwrap_or_else(|_| start.to_path_buf());
    let workspaces: Vec<Workspace> = workspaces.into_iter().filter_map(canonical).collect();
    let Some(workspace) = workspace_for(&workspaces, &start) else {
        bail!("{not_found}, or in a workspace in {USER_CONFIG}");
    };
    let tree = workspace.tree.as_path();
    fs::canonicalize(tree)
        .ok()
        .filter(|tree| tree.join(config::FILE_NAME).is_file())
        .ok_or_else(|| {
            let tree = shown(tree, &home);
            anyhow!(
                "{USER_CONFIG}: workspace tree `{tree}` has no {}",
                config::FILE_NAME
            )
        })
}

/// The user config's workspaces, or `None` when there's no user config.
fn read_workspaces(home: &Path) -> Result<Option<Vec<Workspace>>> {
    let text = match fs::read_to_string(home.join(workspace::FILE_PATH)) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context(format!("can't read {USER_CONFIG}")),
    };
    let workspaces = workspace::parse(&text, home).map_err(|error| match error.line {
        Some(line) => anyhow!("{USER_CONFIG}:{line}: {}", error.message),
        None => anyhow!("{USER_CONFIG}: {}", error.message),
    })?;
    Ok(Some(workspaces))
}

// A workspace folder that doesn't exist can't contain the current folder, so it's skipped.
fn canonical(workspace: Workspace) -> Option<Workspace> {
    let path = fs::canonicalize(workspace.path.as_path()).ok()?;
    Some(Workspace {
        path: AbsPath::new(path)?,
        tree: workspace.tree,
    })
}

fn shown(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
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
