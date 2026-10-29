//! Sorts a tree's files by where they are: node files, nested trees and Markdown.

use std::collections::BTreeSet;

use crate::config::{self, TreeConfig};
use crate::node::NodeLocation;
use crate::paths::{RepoPath, TreePath};

/// A folder named this holds decision records, so it and everything under it is never a node.
const RECORDS_FOLDER: &str = "decisions";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    /// Files in a node file's position, before their frontmatter is read.
    pub node_files: Vec<NodeFile>,
    /// The outermost folders under the root that start their own tree.
    pub nested_trees: Vec<TreePath>,
    /// Markdown in this tree, outside nested trees, including decision records.
    pub markdown: Vec<TreePath>,
    /// The Markdown that `check` inspects: the same, without decision records.
    pub checked: Vec<TreePath>,
    /// Every file in this tree outside nested trees and decision records folders.
    pub files: Vec<TreePath>,
    /// Decision records: the Markdown in decision records folders.
    pub records: Vec<TreePath>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeFile {
    pub path: TreePath,
    pub location: NodeLocation,
}

pub fn layout(config: &TreeConfig, tree_root: &RepoPath, paths: &BTreeSet<RepoPath>) -> Layout {
    let in_tree: Vec<TreePath> = paths.iter().filter_map(|p| p.in_tree(tree_root)).collect();
    let nested_trees = nested_trees(&in_tree);
    let files: Vec<TreePath> = in_tree
        .into_iter()
        .filter(|file| !nested_trees.iter().any(|tree| file.is_within(tree)))
        .collect();
    let root_file = config.root_node_file();
    let node_files = files
        .iter()
        .filter(|file| !in_records_folder(file))
        .filter_map(|file| {
            let location = node_location(file, &root_file)?;
            Some(NodeFile {
                path: file.clone(),
                location,
            })
        })
        .collect();
    let markdown: Vec<TreePath> = files
        .iter()
        .filter(|file| file.as_str().ends_with(".md"))
        .cloned()
        .collect();
    let (records, checked) = markdown.iter().cloned().partition(in_records_folder);
    let files = files
        .into_iter()
        .filter(|file| !in_records_folder(file))
        .collect();
    Layout {
        node_files,
        nested_trees,
        markdown,
        checked,
        files,
        records,
    }
}

/// The outermost decision records folder that holds `file`, or `None` when none does.
pub fn records_folder(file: &TreePath) -> Option<TreePath> {
    let mut outermost = None;
    let mut folder = file.parent();
    while let Some(current) = folder {
        if current.file_name() == RECORDS_FOLDER {
            outermost = Some(current.clone());
        }
        folder = current.parent();
    }
    outermost
}

fn nested_trees(files: &[TreePath]) -> Vec<TreePath> {
    let roots: Vec<TreePath> = files
        .iter()
        .filter(|file| file.file_name() == config::FILE_NAME)
        .filter_map(TreePath::parent)
        .filter(|folder| !folder.as_str().is_empty())
        .collect();
    roots
        .iter()
        .filter(|folder| !roots.iter().any(|outer| folder.is_within(outer)))
        .cloned()
        .collect()
}

fn in_records_folder(file: &TreePath) -> bool {
    let folder = file.parent().unwrap_or_default();
    folder
        .as_str()
        .split('/')
        .any(|segment| segment == RECORDS_FOLDER)
}

fn node_location(file: &TreePath, root_file: &TreePath) -> Option<NodeLocation> {
    if file == root_file {
        return Some(NodeLocation::Root);
    }
    let folder = file.parent().filter(|folder| !folder.as_str().is_empty())?;
    let named_after_folder = file.file_name() == format!("{}.md", folder.file_name());
    named_after_folder.then_some(NodeLocation::Folder(folder))
}
