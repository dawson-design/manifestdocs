//! What `check` reads, and the findings it returns.

use std::collections::{BTreeMap, BTreeSet};

use crate::config::{self, TreeConfig};
use crate::finding::{Field, Finding, Problem, Severity};
use crate::frontmatter::{self, Split};
use crate::id::NodeId;
use crate::layout::{Layout, NodeFile, layout};
use crate::link::{LinkTarget, Targets, resolve};
use crate::markdown::outline;
use crate::node::{NodeLocation, ParsedNode, read_node_file};
use crate::paths::{RepoPath, TreePath};
use crate::tree::tree_findings;

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

/// The Markdown files `check` reads: every one in the tree, outside nested trees.
pub fn markdown_to_read(
    config: &TreeConfig,
    tree_root: &RepoPath,
    paths: &BTreeSet<RepoPath>,
) -> Vec<RepoPath> {
    let layout = layout(config, tree_root, paths);
    let markdown = layout.markdown.iter();
    markdown
        .filter_map(|file| tree_root.join(file.as_str()))
        .collect()
}

pub fn check(config: &TreeConfig, snapshot: &Snapshot) -> Report {
    let layout = layout(config, &snapshot.tree_root, &snapshot.paths);
    let nodes: Vec<ParsedNode> = layout
        .node_files
        .iter()
        .filter_map(|file| read(config, snapshot, file))
        .collect();
    let mut findings = layout_findings(config, &layout, &nodes);
    findings.extend(nodes.iter().flat_map(|node| node.findings.iter().cloned()));
    findings.extend(tree_findings(&nodes));
    findings.extend(stray_node_ids(snapshot, &layout));
    findings.extend(broken_links(snapshot, &layout));
    findings.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Report {
        nodes: nodes.iter().filter(|node| node.id.is_some()).count(),
        findings,
    }
}

fn read(config: &TreeConfig, snapshot: &Snapshot, file: &NodeFile) -> Option<ParsedNode> {
    let text = markdown(snapshot, &file.path)?;
    read_node_file(&file.path, file.location.clone(), text, config)
}

fn markdown<'a>(snapshot: &'a Snapshot, file: &TreePath) -> Option<&'a str> {
    let path = snapshot.tree_root.join(file.as_str())?;
    snapshot.markdown.get(&path).map(String::as_str)
}

fn broken_links(snapshot: &Snapshot, layout: &Layout) -> Vec<Finding> {
    let targets = Targets::new(&snapshot.paths);
    let checked = layout.checked.iter();
    checked
        .flat_map(|file| broken_links_in(snapshot, &targets, file))
        .collect()
}

fn broken_links_in(snapshot: &Snapshot, targets: &Targets, file: &TreePath) -> Vec<Finding> {
    let (Some(text), Some(from)) = (
        markdown(snapshot, file),
        snapshot.tree_root.join(file.as_str()),
    ) else {
        return Vec::new();
    };
    let (body, first_line) = frontmatter::body(text);
    let links = outline(body, first_line).links.into_iter();
    links
        .filter(|link| match resolve(&link.target, &from) {
            Some(LinkTarget::External(_)) => false,
            Some(LinkTarget::Local { path, .. }) => !targets.exists(&path),
            None => true,
        })
        .map(|link| Finding {
            file: file.clone(),
            line: Some(link.line),
            problem: Problem::BrokenLink(link.target),
        })
        .collect()
}

// A node ID outside a node file usually means a folder was renamed without its node file.
fn stray_node_ids(snapshot: &Snapshot, layout: &Layout) -> Vec<Finding> {
    let node_files: BTreeSet<&TreePath> = layout.node_files.iter().map(|f| &f.path).collect();
    let others = layout
        .checked
        .iter()
        .filter(|file| !node_files.contains(file));
    others
        .filter_map(|file| {
            let Split::Present(frontmatter) = frontmatter::split(markdown(snapshot, file)?) else {
                return None;
            };
            let key = frontmatter.key(Field::Id.name())?;
            let id = NodeId::parse(key.value.trim_matches(['"', '\'']))?;
            Some(Finding {
                file: file.clone(),
                line: Some(key.line),
                problem: Problem::NodeIdOutsideNodeFile(id),
            })
        })
        .collect()
}

fn layout_findings(config: &TreeConfig, layout: &Layout, nodes: &[ParsedNode]) -> Vec<Finding> {
    let nested = layout.nested_trees.iter().map(|folder| Finding {
        file: folder.clone(),
        line: None,
        problem: Problem::NestedTreeSkipped,
    });
    let has_root = nodes
        .iter()
        .any(|node| node.location == NodeLocation::Root && node.id.is_some());
    let no_root = (!has_root).then(|| Finding {
        file: TreePath::new(config::FILE_NAME).unwrap_or_default(),
        line: None,
        problem: Problem::NoRootNode {
            expected: config.root_node_file(),
        },
    });
    nested.chain(no_root).collect()
}
