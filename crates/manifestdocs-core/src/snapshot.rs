//! What the I/O side reads for the pure core.

use std::collections::{BTreeMap, BTreeSet};

use crate::config::TreeConfig;
use crate::layout::NodeFile;
use crate::node::{ParsedNode, read_node_file};
use crate::paths::{RepoPath, TreePath};

/// The files the I/O side reads for the pure core.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// The tree's root folder. Empty when the tree is the repo root.
    pub tree_root: RepoPath,
    /// Every file in the repo that git doesn't ignore.
    pub paths: BTreeSet<RepoPath>,
    /// The Markdown the core named for this command: every Markdown file in the tree, plus
    /// Markdown outside it that links point into with an anchor, for `check`, and the node
    /// files for the other commands. A path that isn't here wasn't asked for.
    pub markdown: BTreeMap<RepoPath, String>,
}

impl Snapshot {
    /// The text of `file`, a path in the tree, when it was read.
    pub(crate) fn tree_markdown(&self, file: &TreePath) -> Option<&str> {
        let path = self.tree_root.join(file.as_str())?;
        self.markdown.get(&path).map(String::as_str)
    }

    pub(crate) fn read_node(&self, config: &TreeConfig, file: &NodeFile) -> Option<ParsedNode> {
        let text = self.tree_markdown(&file.path)?;
        read_node_file(&file.path, file.location.clone(), text, config)
    }
}
