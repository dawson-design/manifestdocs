//! `stale`: the open issues linked to each node, the docs version each was planned against,
//! and the files each node owns, for the binary to compare in the docs clone's history.

use std::collections::{BTreeMap, BTreeSet};

use crate::config;
use crate::id::NodeId;
use crate::layout::records_folder;
use crate::paths::{RepoPath, TreePath};
use crate::refs::IssueRef;
use crate::tree::{Entry, Tree};
use crate::work::{
    Fetched, Issue, IssueBody, IssueKey, IssueLinks, IssueState, SubIssue, Takeovers, Unreadable,
    takeovers,
};

/// The docs version an issue was planned against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Baseline {
    /// The commit its `spec-ref:` line names, as written.
    SpecRef(String),
    /// No `spec-ref:` line: the docs' default branch when the issue was created, at this
    /// RFC 3339 time.
    Created(String),
}

/// An issue linked to a node: listed in its `work`, a sub-issue of one, or naming the node in
/// a `feature:` line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Linked<'a> {
    Open {
        node: &'a Entry,
        issue: &'a Issue,
        baseline: Baseline,
    },
    /// An issue `gh` couldn't read. `reference` is `None` for a sub-issue GitHub listed but
    /// couldn't return.
    Unread {
        node: &'a Entry,
        reference: Option<&'a IssueRef>,
        why: &'a Unreadable,
    },
}

/// The issues to read from GitHub, with their sub-issues: every node's `work`, and each
/// issue in `named` whose `feature:` line names a node in the tree.
pub fn issues_to_read(tree: &Tree, named: &[IssueBody]) -> Vec<IssueRef> {
    let nodes = tree
        .entries
        .values()
        .filter_map(|entry| entry.node.as_ref().ok());
    let work = nodes.flat_map(|node| node.work.iter().cloned());
    let by_node = named_by_node(tree, named);
    let named = by_node.into_values().flatten().cloned();
    work.chain(named).collect()
}

/// The issues in `named` whose `feature:` line names a node in the tree, by node.
fn named_by_node<'a>(tree: &Tree, named: &'a [IssueBody]) -> BTreeMap<NodeId, Vec<&'a IssueRef>> {
    let ids: BTreeSet<&NodeId> = tree.entries.values().map(|entry| &entry.id).collect();
    let mut by_node: BTreeMap<NodeId, Vec<&IssueRef>> = BTreeMap::new();
    for found in named {
        let Some(id) = IssueLinks::parse(&found.body).feature else {
            continue;
        };
        if ids.contains(&id) {
            by_node.entry(id).or_default().push(&found.issue);
        }
    }
    by_node
}

/// Each node's linked issues, nodes in outline order, each node's `work` then the issues that
/// name it, each followed by its sub-issues. A sub-issue that a deeper node lists in `work`
/// belongs to that node, with its sub-issues. Closed issues are left out, and an issue
/// reached twice for one node is listed once.
pub fn linked_issues<'a>(
    tree: &'a Tree,
    fetched: &'a Fetched,
    named: &'a [IssueBody],
) -> Vec<Linked<'a>> {
    let by_node = named_by_node(tree, named);
    let mut linked = Vec::new();
    for (_, entry) in tree.outline(&TreePath::default(), None) {
        let Ok(node) = &entry.node else {
            continue;
        };
        let named = by_node.get(&entry.id).into_iter().flatten().copied();
        let walk = Walk {
            node: entry,
            fetched,
            takeovers: takeovers(tree, entry, node),
        };
        let mut seen = BTreeSet::new();
        for root in node.work.iter().chain(named) {
            walk.visit(root, true, &mut seen, &mut linked);
        }
    }
    linked
}

struct Walk<'a> {
    node: &'a Entry,
    fetched: &'a Fetched,
    takeovers: Takeovers<'a>,
}

impl<'a> Walk<'a> {
    fn visit(
        &self,
        reference: &'a IssueRef,
        root: bool,
        seen: &mut BTreeSet<IssueKey>,
        linked: &mut Vec<Linked<'a>>,
    ) {
        let key = IssueKey::from(reference);
        if (!root && self.takeovers.contains_key(&key)) || !seen.insert(key.clone()) {
            return;
        }
        // `fetch_hierarchy` reads every issue it's given, so a missing one isn't expected.
        match self.fetched.get(&key) {
            Some(Ok(issue)) => self.push(issue, seen, linked),
            Some(Err(why)) => linked.push(self.unread(Some(reference), why)),
            None => {}
        }
    }

    /// `issue`, when it's open, then its sub-issues.
    fn push(&self, issue: &'a Issue, seen: &mut BTreeSet<IssueKey>, linked: &mut Vec<Linked<'a>>) {
        if issue.state == IssueState::Open {
            linked.push(Linked::Open {
                node: self.node,
                issue,
                baseline: baseline(issue),
            });
        }
        for sub in &issue.sub_issues {
            match sub {
                SubIssue::Issue(sub) => self.visit(sub, false, seen, linked),
                SubIssue::Unreadable(why) => linked.push(self.unread(None, why)),
            }
        }
    }

    fn unread(&self, reference: Option<&'a IssueRef>, why: &'a Unreadable) -> Linked<'a> {
        Linked::Unread {
            node: self.node,
            reference,
            why,
        }
    }
}

fn baseline(issue: &Issue) -> Baseline {
    match IssueLinks::parse(&issue.body).spec_ref {
        Some(spec_ref) => Baseline::SpecRef(spec_ref),
        None => Baseline::Created(issue.created_at.clone()),
    }
}

/// The files a node owns, from the repo root: everything under its folder except deeper
/// nodes' folders, nested trees, decision records folders and the tree's config. Listing a
/// folder rather than its files catches files added or deleted since.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Watched {
    pub folder: RepoPath,
    pub excluded: Vec<RepoPath>,
}

/// The files `entry` owns. `paths` are the repo's files, which locate its records folders.
pub fn watched(
    tree: &Tree,
    entry: &Entry,
    tree_root: &RepoPath,
    paths: &BTreeSet<RepoPath>,
) -> Watched {
    let folder = entry.location.folder();
    let nodes = tree.entries.values().map(|other| other.location.folder());
    let in_tree = paths.iter().filter_map(|path| path.in_tree(tree_root));
    let records = in_tree.filter_map(|file| records_folder(&file));
    let config = TreePath::new(config::FILE_NAME);
    let excluded: BTreeSet<TreePath> = nodes
        .chain(tree.nested_trees.iter().cloned())
        .chain(records)
        .chain(config)
        .filter(|path| path.is_within(&folder))
        .collect();
    let repo_path = |path: &TreePath| tree_root.join(path.as_str());
    Watched {
        folder: repo_path(&folder).unwrap_or_else(|| tree_root.clone()),
        excluded: excluded.iter().filter_map(repo_path).collect(),
    }
}

/// Whether `spec_ref` can be a commit ID: 4 to 64 hex digits, so a branch name or an
/// expression such as `HEAD~1` isn't read as one.
pub fn is_commit_id(spec_ref: &str) -> bool {
    let hex = spec_ref.bytes().all(|byte| byte.is_ascii_hexdigit());
    hex && (4..=64).contains(&spec_ref.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_hex_commit_ids_as_spec_refs() {
        for spec_ref in ["4f1c", "4f1c2ab", &"a".repeat(40), &"A".repeat(64)] {
            assert!(is_commit_id(spec_ref), "{spec_ref}");
        }
        for spec_ref in ["4f1", "main", "HEAD~1", "4f1c2ab^", &"a".repeat(65)] {
            assert!(!is_commit_id(spec_ref), "{spec_ref}");
        }
    }
}
