//! `manifestdocs check`: validate the tree around the current folder, or the tree that
//! contains a given file.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow, bail};
use manifestdocs_core::check::check;

use crate::files;
use crate::locate::locate;
use crate::report::{relative, render, write_stdout};

pub fn run(path: Option<&str>) -> Result<ExitCode> {
    let cwd = env::current_dir().context("can't read the current folder")?;
    let start = match path {
        Some(path) => start_folder(path, &cwd)?,
        None => cwd.clone(),
    };
    let roots = locate(&start)?;
    if let Some(path) = path {
        in_tree(path, &cwd, &roots.tree_root)?;
    }
    let config = files::read_config(&roots.tree_root, &cwd)?;
    let mut snapshot = files::snapshot(&roots, &config, &cwd)?;
    files::read_anchor_targets(&mut snapshot, &roots, &config, &cwd)?;
    let report = check(&config, &snapshot);
    write_stdout(&render(&report, &roots.tree_root, &cwd))?;
    Ok(if report.has_errors() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

/// The folder the tree is looked for from: `path` itself, or the folder a file is in.
fn start_folder(path: &str, cwd: &Path) -> Result<PathBuf> {
    let path =
        fs::canonicalize(cwd.join(path)).map_err(|_| anyhow!("no file or folder `{path}`"))?;
    match path.is_dir() {
        true => Ok(path),
        false => Ok(path.parent().map_or(path.clone(), Path::to_path_buf)),
    }
}

// A pointer can lead to a tree that doesn't contain a file, such as a code repo's README. A
// folder is checked the way running `check` in it would be, so `check .` matches `check`.
fn in_tree(path: &str, cwd: &Path, tree_root: &Path) -> Result<()> {
    let file =
        fs::canonicalize(cwd.join(path)).map_err(|_| anyhow!("no file or folder `{path}`"))?;
    let root = fs::canonicalize(tree_root).context("can't read the tree's folder")?;
    if !file.is_dir() && !file.starts_with(&root) {
        let tree = relative(tree_root, cwd);
        bail!("`{path}` isn't in a tree: the nearest tree is `{tree}`");
    }
    Ok(())
}
