//! `manifestdocs tree`: an outline of the nodes, with their IDs, statuses and files.

use std::env;
use std::fmt::Write as _;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use manifestdocs_core::node::NodeStatus;
use manifestdocs_core::paths::TreePath;
use manifestdocs_core::tree::{Entry, Tree};

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
        let _ = writeln!(out, "{indent}{}  {file}", heading(&tree, entry));
    }
    write_stdout(&out)?;
    Ok(ExitCode::SUCCESS)
}

/// A node's name, ID and status, such as `Recipe search (fd-7c2e41, approved)`.
pub fn heading(tree: &Tree, entry: &Entry) -> String {
    let status = match &entry.node {
        Ok(node) => match &node.status {
            NodeStatus::Active(status) => status.as_str(),
            NodeStatus::Superseded { .. } => "superseded",
        },
        Err(_) => "has problems",
    };
    format!("{} ({}, {status})", name(tree, entry), entry.id)
}

/// A node's title, or, when its frontmatter has errors, its folder's name or the tree's name.
pub fn name<'a>(tree: &'a Tree, entry: &'a Entry) -> &'a str {
    match (&entry.node, folder_name(&entry.file)) {
        (Ok(node), _) => node.title.as_str(),
        (Err(_), "") => tree.config.name.as_str(),
        (Err(_), folder) => folder,
    }
}

fn folder_name(file: &TreePath) -> &str {
    let folder = file
        .as_str()
        .rsplit_once('/')
        .map_or("", |(folder, _)| folder);
    folder.rsplit_once('/').map_or(folder, |(_, name)| name)
}

// A node file stands for its node's folder.
fn start_folder(tree: &Tree, arg: &str, cwd: &Path, tree_root: &Path) -> Result<TreePath> {
    let path = tree_path(arg, cwd, tree_root)?;
    if let Some(entry) = tree.entries.get(&path) {
        return Ok(entry.location.folder());
    }
    if tree_root.join(path.as_str()).is_dir() {
        return Ok(path);
    }
    bail!("`{arg}` isn't a node file or a folder")
}
