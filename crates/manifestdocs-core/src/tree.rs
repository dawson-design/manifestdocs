//! A tree: its config and the nodes found under its root, and the rules across nodes.

use std::collections::BTreeMap;

use crate::config::TreeConfig;
use crate::finding::{Field, Finding, Problem};
use crate::id::NodeId;
use crate::node::{Node, NodeStatus, ParsedNode};
use crate::paths::TreePath;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tree {
    pub config: TreeConfig,
    pub nodes: BTreeMap<NodeId, Node>,
    pub skipped_trees: Vec<TreePath>,
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
