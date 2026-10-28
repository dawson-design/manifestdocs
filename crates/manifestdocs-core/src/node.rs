//! Feature nodes: a folder's node file, parsed from its frontmatter.

use crate::config::StatusName;
use crate::id::NodeId;
use crate::paths::TreePath;
use crate::refs::{IssueRef, RepoRef};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    pub location: NodeLocation,
    pub title: NonEmptyString,
    pub status: NodeStatus,
    pub summary: NonEmptyString,
    pub repos: Vec<RepoRef>,
    pub work: Vec<IssueRef>,
    pub body: String,
}

/// Where a node is, taken from its file's place in the tree, never from frontmatter.
/// `Folder` always names a folder below the root; the root itself is `Root`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeLocation {
    Root,
    Folder(TreePath),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeStatus {
    Active(StatusName),
    Superseded { by: NodeId },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    pub fn new(text: String) -> Option<Self> {
        (!text.trim().is_empty()).then_some(Self(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
