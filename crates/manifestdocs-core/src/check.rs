//! What `check` reads, and the findings it returns.

use std::collections::{BTreeMap, BTreeSet};

use crate::config::{self, TreeConfig};
use crate::finding::{Finding, Problem, Severity};
use crate::layout::{Layout, layout};
use crate::node::NodeLocation;
use crate::paths::{RepoPath, TreePath};

/// The files the I/O side reads for the pure core.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// The tree's root folder. Empty when the tree is the repo root.
    pub tree_root: RepoPath,
    /// Every file in the repo that git doesn't ignore.
    pub paths: BTreeSet<RepoPath>,
    /// Every Markdown file under the tree root, plus Markdown outside it that links point into.
    pub markdown: BTreeMap<RepoPath, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub nodes: usize,
    /// Sorted by file, then line.
    pub findings: Vec<Finding>,
}

impl Report {
    pub fn has_errors(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.problem.severity() == Severity::Error)
    }
}

pub fn check(config: &TreeConfig, snapshot: &Snapshot) -> Report {
    let layout = layout(config, snapshot);
    let mut findings = layout_findings(config, &layout);
    findings.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Report {
        nodes: layout.node_files.len(),
        findings,
    }
}

fn layout_findings(config: &TreeConfig, layout: &Layout) -> Vec<Finding> {
    let nested = layout.nested_trees.iter().map(|folder| Finding {
        file: folder.clone(),
        line: None,
        problem: Problem::NestedTreeSkipped,
    });
    let has_root = layout
        .node_files
        .iter()
        .any(|file| file.location == NodeLocation::Root);
    let no_root = (!has_root).then(|| Finding {
        file: TreePath::new(config::FILE_NAME).unwrap_or_default(),
        line: None,
        problem: Problem::NoRootNode {
            expected: config.root_node_file(),
        },
    });
    nested.chain(no_root).collect()
}
