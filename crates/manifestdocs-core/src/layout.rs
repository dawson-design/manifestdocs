//! Sorts a tree's files by where they are: node files and nested trees.

use crate::check::Snapshot;
use crate::config::{self, TreeConfig};
use crate::node::NodeLocation;
use crate::paths::TreePath;

/// A folder named this holds decision records, so it and everything under it is never a node.
const RECORDS_FOLDER: &str = "decisions";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    /// Files in a node file's position, before their frontmatter is read.
    pub node_files: Vec<NodeFile>,
    /// The outermost folders under the root that start their own tree.
    pub nested_trees: Vec<TreePath>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeFile {
    pub path: TreePath,
    pub location: NodeLocation,
}

pub fn layout(config: &TreeConfig, snapshot: &Snapshot) -> Layout {
    let files: Vec<TreePath> = snapshot
        .paths
        .iter()
        .filter_map(|path| path.in_tree(&snapshot.tree_root))
        .collect();
    let nested_trees = nested_trees(&files);
    let root_file = config.root_node_file();
    let node_files = files
        .iter()
        .filter(|file| !nested_trees.iter().any(|tree| file.is_within(tree)))
        .filter(|file| !in_records_folder(file))
        .filter_map(|file| {
            let location = node_location(file, &root_file)?;
            Some(NodeFile {
                path: file.clone(),
                location,
            })
        })
        .collect();
    Layout {
        node_files,
        nested_trees,
    }
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
