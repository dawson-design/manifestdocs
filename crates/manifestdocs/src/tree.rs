//! `manifestdocs tree`: an outline of the nodes, with their IDs, statuses and files.

use std::env;
use std::fmt::Write as _;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use manifestdocs_core::paths::TreePath;
use manifestdocs_core::tree::Tree;

use crate::files;
use crate::locate::{locate, tree_path};
use crate::report::{relative, write_stdout};

pub fn run(path: Option<&str>, depth: Option<usize>) -> Result<ExitCode> {
    let cwd = env::current_dir().context("can't read the current folder")?;
    let roots = locate(&cwd)?;
    let tree = files::tree(&roots, &cwd)?;
    let folder = match path {
        Some(arg) => start_folder(&tree, arg, &cwd, &roots.tree_root)?,
        None => TreePath::default(),
    };
    let lines = tree.outline(&folder, depth);
    let top = lines.first().map_or(0, |(level, _)| *level);
    let mut out = String::new();
    for (level, entry) in lines {
        let indent = "  ".repeat(level - top);
        let file = relative(&roots.tree_root.join(entry.file.as_str()), &cwd);
        let _ = writeln!(out, "{indent}{}  {file}", tree.heading(entry));
    }
    write_stdout(&out)?;
    Ok(ExitCode::SUCCESS)
}

// A node file stands for its node's folder.
fn start_folder(tree: &Tree, arg: &str, cwd: &Path, tree_root: &Path) -> Result<TreePath> {
    let path = path_in_tree(tree, arg, cwd, tree_root)?;
    if let Some(entry) = tree.entries.get(&path) {
        return Ok(entry.location.folder());
    }
    if tree_root.join(path.as_str()).is_dir() {
        return Ok(path);
    }
    bail!("`{arg}` isn't a node file or a folder")
}

/// `arg` as a path in this tree. A nested tree's nodes aren't this tree's, so a path into one
/// is an error that names it.
pub fn path_in_tree(tree: &Tree, arg: &str, cwd: &Path, tree_root: &Path) -> Result<TreePath> {
    let path = tree_path(arg, cwd, tree_root)?;
    let mut nested = tree.nested_trees.iter();
    if let Some(nested) = nested.find(|n| path == **n || path.is_within(n)) {
        let folder = relative(&tree_root.join(nested.as_str()), cwd);
        bail!("`{arg}` is in a separate tree, at `{folder}`; run the command there");
    }
    Ok(path)
}
