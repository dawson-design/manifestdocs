//! A tree: its config and the nodes found under its root, and the rules across nodes.

use std::collections::{BTreeMap, BTreeSet};

use crate::check::{Snapshot, read_node};
use crate::config::{self, TreeConfig};
use crate::finding::{Field, Finding, Problem, Severity};
use crate::id::{NodeId, NodeIdPrefix};
use crate::layout::layout;
use crate::node::{Node, NodeLocation, NodeStatus, ParsedNode};
use crate::paths::{RepoPath, TreePath};

/// How a command names one node: by ID or prefix, or by its node file or folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeSelector {
    Id(NodeIdPrefix),
    Path(TreePath),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectError<'a> {
    NoMatch,
    /// More than one node has the ID or prefix, sorted by ID.
    Ambiguous(Vec<&'a Entry>),
    /// The path is in the tree but isn't a node file or node folder.
    NotANode,
}

/// The nodes in a tree and how they relate, as `tree` and `show` read them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tree {
    pub config: TreeConfig,
    /// Keyed by node file, since two files can share an ID.
    pub entries: BTreeMap<TreePath, Entry>,
    /// Each node file's parent's node file. No entry: the root node, or a top-level node in a
    /// tree without one.
    pub parents: BTreeMap<TreePath, TreePath>,
    /// Each supporting doc's owner's node file. No entry: no node owns it.
    pub owners: BTreeMap<TreePath, TreePath>,
}

/// A node file whose `id` parses, which makes it a node whether or not the rest is valid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub file: TreePath,
    pub location: NodeLocation,
    pub id: NodeId,
    /// The node, or the errors in its frontmatter, which stop `tree` and `show` using it.
    pub node: Result<Node, Vec<Finding>>,
}

/// The files `read_tree` reads: the files in a node file's position. Supporting docs are
/// only listed.
pub fn node_files_to_read(
    config: &TreeConfig,
    tree_root: &RepoPath,
    paths: &BTreeSet<RepoPath>,
) -> Vec<RepoPath> {
    let layout = layout(config, tree_root, paths);
    let node_files = layout.node_files.iter();
    node_files
        .filter_map(|file| tree_root.join(file.path.as_str()))
        .collect()
}

pub fn read_tree(config: &TreeConfig, snapshot: &Snapshot) -> Tree {
    let layout = layout(config, &snapshot.tree_root, &snapshot.paths);
    let entries: BTreeMap<TreePath, Entry> = layout
        .node_files
        .iter()
        .filter_map(|file| entry(read_node(config, snapshot, file)?))
        .map(|entry| (entry.file.clone(), entry))
        .collect();
    let folders: BTreeMap<TreePath, TreePath> = entries
        .values()
        .map(|entry| (entry.location.folder(), entry.file.clone()))
        .collect();
    let parents = entries
        .values()
        .filter_map(|entry| {
            let parent = nearest_node(&folders, entry.location.folder().parent()?)?;
            Some((entry.file.clone(), parent))
        })
        .collect();
    let config_file = TreePath::new(config::FILE_NAME).unwrap_or_default();
    let owners = layout
        .files
        .into_iter()
        .filter(|file| !entries.contains_key(file) && *file != config_file)
        .filter_map(|file| {
            let owner = nearest_node(&folders, file.parent()?)?;
            Some((file, owner))
        })
        .collect();
    Tree {
        config: config.clone(),
        entries,
        parents,
        owners,
    }
}

fn entry(parsed: ParsedNode) -> Option<Entry> {
    let id = parsed.id?;
    let errors: Vec<Finding> = parsed
        .findings
        .into_iter()
        .filter(|finding| finding.problem.severity() == Severity::Error)
        .collect();
    let node = match parsed.node {
        Some(node) if errors.is_empty() => Ok(node),
        _ => Err(errors),
    };
    Some(Entry {
        file: parsed.file,
        location: parsed.location,
        id,
        node,
    })
}

/// The node file of the nearest node at or above `folder`.
fn nearest_node(folders: &BTreeMap<TreePath, TreePath>, folder: TreePath) -> Option<TreePath> {
    let mut folder = Some(folder);
    while let Some(current) = folder {
        if let Some(file) = folders.get(&current) {
            return Some(file.clone());
        }
        folder = current.parent();
    }
    None
}

impl Tree {
    /// The nodes at or below `folder`, depth first, each with its level below `folder`'s own
    /// node. When `folder` has no node of its own, its top-level nodes are level 1. `depth`
    /// limits the levels shown.
    pub fn outline(&self, folder: &TreePath, depth: Option<usize>) -> Vec<(usize, &Entry)> {
        let mut lines = Vec::new();
        match self.entry_at(folder) {
            Some(entry) => self.push_outline(entry, 0, depth, &mut lines),
            None => {
                for entry in self.top_within(folder) {
                    self.push_outline(entry, 1, depth, &mut lines);
                }
            }
        }
        lines
    }

    fn push_outline<'a>(
        &'a self,
        entry: &'a Entry,
        level: usize,
        depth: Option<usize>,
        lines: &mut Vec<(usize, &'a Entry)>,
    ) {
        if depth.is_some_and(|depth| level > depth) {
            return;
        }
        lines.push((level, entry));
        for child in self.children(&entry.file) {
            self.push_outline(child, level + 1, depth, lines);
        }
    }

    pub fn select(&self, selector: &NodeSelector) -> Result<&Entry, SelectError<'_>> {
        match selector {
            NodeSelector::Id(prefix) => {
                let mut matches: Vec<&Entry> = self
                    .entries
                    .values()
                    .filter(|entry| prefix.matches(&entry.id))
                    .collect();
                matches.sort_by_key(|entry| &entry.id);
                match matches.as_slice() {
                    [] => Err(SelectError::NoMatch),
                    [entry] => Ok(entry),
                    _ => Err(SelectError::Ambiguous(matches)),
                }
            }
            NodeSelector::Path(path) => {
                let entry = self.entries.get(path).or_else(|| self.entry_at(path));
                entry.ok_or(SelectError::NotANode)
            }
        }
    }

    /// The node whose folder is `folder`.
    pub fn entry_at(&self, folder: &TreePath) -> Option<&Entry> {
        self.entries
            .values()
            .find(|entry| entry.location.folder() == *folder)
    }

    /// The nodes whose parent is the node in `file`, in folder order.
    pub fn children(&self, file: &TreePath) -> Vec<&Entry> {
        let children = self.entries.values();
        sorted(children.filter(|entry| self.parents.get(&entry.file) == Some(file)))
    }

    /// The parent of the node in `file`.
    pub fn parent(&self, file: &TreePath) -> Option<&Entry> {
        self.entries.get(self.parents.get(file)?)
    }

    /// The files the node in `file` owns, in path order.
    pub fn supporting_docs(&self, file: &TreePath) -> Vec<&TreePath> {
        let owned = self.owners.iter();
        owned
            .filter(|(_, owner)| *owner == file)
            .map(|(doc, _)| doc)
            .collect()
    }

    // The nodes in `folder` whose parent, if any, is outside it.
    fn top_within(&self, folder: &TreePath) -> Vec<&Entry> {
        let inside = |path: &TreePath| path == folder || path.is_within(folder);
        let top = self.entries.values().filter(|entry| {
            let parent = self.parent(&entry.file);
            inside(&entry.location.folder())
                && !parent.is_some_and(|p| inside(&p.location.folder()))
        });
        sorted(top)
    }
}

// Folder order compares segment by segment, so `recipes/search` comes before `recipes-archive`.
fn sorted<'a>(entries: impl Iterator<Item = &'a Entry>) -> Vec<&'a Entry> {
    let mut entries: Vec<&Entry> = entries.collect();
    entries.sort_by_cached_key(|entry| {
        let folder = entry.location.folder();
        folder
            .as_str()
            .split('/')
            .map(str::to_owned)
            .collect::<Vec<_>>()
    });
    entries
}

/// Duplicate IDs, and replacements that name no node in the tree.
pub fn tree_findings(nodes: &[ParsedNode]) -> Vec<Finding> {
    let index = id_index(nodes);
    let mut findings = duplicate_ids(&index);
    findings.extend(missing_replacements(nodes, &index));
    findings
}

// Every node file whose ID parses, valid or not, so one bad field doesn't hide a node.
fn id_index(nodes: &[ParsedNode]) -> BTreeMap<&NodeId, Vec<&ParsedNode>> {
    let mut index: BTreeMap<&NodeId, Vec<&ParsedNode>> = BTreeMap::new();
    for node in nodes {
        if let Some(id) = &node.id {
            index.entry(id).or_default().push(node);
        }
    }
    index
}

// Each file that shares an ID names another file that uses it.
fn duplicate_ids(index: &BTreeMap<&NodeId, Vec<&ParsedNode>>) -> Vec<Finding> {
    let shared = index.values().filter(|files| files.len() > 1);
    shared
        .flat_map(|files| {
            files.iter().map(|node| {
                let other = files.iter().find(|other| other.file != node.file);
                let also_in = other.map(|other| other.file.clone()).unwrap_or_default();
                finding(node, Field::Id, Problem::DuplicateId { also_in })
            })
        })
        .collect()
}

fn missing_replacements(
    nodes: &[ParsedNode],
    index: &BTreeMap<&NodeId, Vec<&ParsedNode>>,
) -> Vec<Finding> {
    nodes
        .iter()
        .filter_map(|parsed| match &parsed.node.as_ref()?.status {
            NodeStatus::Superseded { by } if !index.contains_key(by) => {
                let problem = Problem::ReplacementNotFound(by.clone());
                Some(finding(parsed, Field::SupersededBy, problem))
            }
            _ => None,
        })
        .collect()
}

fn finding(node: &ParsedNode, field: Field, problem: Problem) -> Finding {
    Finding {
        file: node.file.clone(),
        line: Some(node.line_of(field)),
        problem,
    }
}
