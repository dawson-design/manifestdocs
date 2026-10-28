//! A tree: its config and the nodes found under its root.

use std::collections::BTreeMap;

use crate::config::TreeConfig;
use crate::id::NodeId;
use crate::node::Node;
use crate::paths::TreePath;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tree {
    pub config: TreeConfig,
    pub nodes: BTreeMap<NodeId, Node>,
    pub skipped_trees: Vec<TreePath>,
}
