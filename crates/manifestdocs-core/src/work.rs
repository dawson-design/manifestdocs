//! Links to work: the node links in issue bodies, `check`'s rules for the issues a tree
//! references and the issues that name its nodes, and the sub-issue hierarchy `work` shows.

use std::collections::{BTreeMap, BTreeSet};
use std::iter;
use std::ops::Range;

use pulldown_cmark::{Event, Parser, Tag};

use crate::check::{Report, Snapshot, read_node};
use crate::config::{self, TreeConfig};
use crate::finding::{Field, Finding, Problem};
use crate::id::NodeId;
use crate::layout::layout;
use crate::node::{Node, ParsedNode};
use crate::paths::TreePath;
use crate::refs::{IssueRef, RepoRef};
use crate::tree::{Entry, Tree};

/// The node an issue names and the docs commit it was planned against, from its body.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IssueLinks {
    pub feature: Option<NodeId>,
    pub spec_ref: Option<String>,
}

impl IssueLinks {
    /// Reads the `feature:` and `spec-ref:` lines in an issue body. A line counts only when it
    /// starts with its key, outside code blocks, and the first line with each key decides. A
    /// `feature:` value that isn't a node ID gives no link.
    pub fn parse(body: &str) -> Self {
        Self {
            feature: value(body, "feature:").and_then(NodeId::parse),
            spec_ref: value(body, "spec-ref:")
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
        }
    }
}

/// The trimmed rest of the first line outside code blocks that starts with `key`.
fn value<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    let code = code_blocks(body);
    let lines = body.split_inclusive('\n').scan(0, |next, line| {
        let start = *next;
        *next += line.len();
        Some((start, line))
    });
    lines
        .filter(|(start, _)| !code.iter().any(|block| block.contains(start)))
        .find_map(|(_, line)| line.strip_prefix(key))
        .map(str::trim)
}

/// The byte ranges of fenced and indented code blocks, fences included.
fn code_blocks(body: &str) -> Vec<Range<usize>> {
    let events = Parser::new(body).into_offset_iter();
    events
        .filter_map(|(event, range)| {
            matches!(event, Event::Start(Tag::CodeBlock(_))).then_some(range)
        })
        .collect()
}

/// An issue a node lists in `work`, at the line of the node's `work` field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkItem {
    pub file: TreePath,
    pub line: u32,
    pub issue: IssueRef,
}

/// The tree's side of `check`'s GitHub rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkLinks {
    /// Every issue the tree's nodes list in `work`, by node file. A node whose frontmatter
    /// has errors lists none, since `check` reports those errors already.
    pub work: Vec<WorkItem>,
    /// The repos whose issues are searched for `feature:` lines: `docs_repo` and every
    /// node's `repos`.
    pub searched_repos: BTreeSet<RepoRef>,
    /// Every node ID in the tree, including those of nodes with frontmatter errors.
    pub node_ids: BTreeSet<NodeId>,
}

/// An issue a search found, with its body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssueBody {
    pub issue: IssueRef,
    pub body: String,
}

/// What `gh` answered for `check`'s GitHub rules.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GitHubAnswers {
    /// Issues in `work` that `gh` can't read.
    pub not_found: BTreeSet<IssueRef>,
    /// The issues the searches of `searched_repos` found.
    pub searched: Vec<IssueBody>,
}

/// What the GitHub rules ask `gh` about. `None` when they don't apply: the tree has no
/// `github` block.
pub fn work_links(config: &TreeConfig, snapshot: &Snapshot) -> Option<WorkLinks> {
    let github = config.github.as_ref()?;
    let layout = layout(config, &snapshot.tree_root, &snapshot.paths);
    let nodes: Vec<ParsedNode> = layout
        .node_files
        .iter()
        .filter_map(|file| read_node(config, snapshot, file))
        .collect();
    let work: Vec<WorkItem> = nodes.iter().flat_map(node_work_items).collect();
    let repos = nodes.iter().filter_map(|parsed| parsed.node.as_ref());
    let repos = repos.flat_map(|node| node.repos.iter().cloned());
    Some(WorkLinks {
        work,
        searched_repos: ignoring_case(iter::once(github.docs_repo.clone()).chain(repos)),
        node_ids: nodes
            .iter()
            .filter_map(|parsed| parsed.id.clone())
            .collect(),
    })
}

// GitHub repo names ignore case, so `Acme/web` and `acme/web` are one repo, searched once
// under the first spelling.
fn ignoring_case(repos: impl Iterator<Item = RepoRef>) -> BTreeSet<RepoRef> {
    let mut by_name: BTreeMap<String, RepoRef> = BTreeMap::new();
    for repo in repos {
        let name = repo.to_string().to_ascii_lowercase();
        by_name.entry(name).or_insert(repo);
    }
    by_name.into_values().collect()
}

fn node_work_items(parsed: &ParsedNode) -> Vec<WorkItem> {
    let line = parsed.line_of(Field::Work);
    let work = parsed.node.iter().flat_map(|node| node.work.iter());
    work.map(|issue| WorkItem {
        file: parsed.file.clone(),
        line,
        issue: issue.clone(),
    })
    .collect()
}

/// `report` with the GitHub rules' findings: an error for each work item `gh` can't read,
/// and a warning on `manifestdocs.yaml` for each searched issue whose `feature:` line is
/// malformed or names a node that isn't in the tree.
pub fn with_github_findings(report: Report, links: &WorkLinks, answers: &GitHubAnswers) -> Report {
    let missing = links
        .work
        .iter()
        .filter(|item| answers.not_found.contains(&item.issue))
        .map(|item| Finding {
            file: item.file.clone(),
            line: Some(item.line),
            problem: Problem::IssueNotFound(item.issue.clone()),
        });
    let mut findings = report.findings;
    findings.extend(missing);
    findings.extend(feature_findings(&links.node_ids, &answers.searched));
    findings.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Report { findings, ..report }
}

fn feature_findings(node_ids: &BTreeSet<NodeId>, searched: &[IssueBody]) -> Vec<Finding> {
    let mut searched: Vec<&IssueBody> = searched.iter().collect();
    searched.sort_by(|a, b| a.issue.cmp(&b.issue));
    // A search can return an issue twice, such as when results shift between pages.
    searched.dedup_by(|a, b| a.issue == b.issue);
    let problems = searched.into_iter().filter_map(|found| {
        let value = value(&found.body, "feature:")?;
        let issue = found.issue.clone();
        match NodeId::parse(value) {
            Some(id) if node_ids.contains(&id) => None,
            Some(id) => Some(Problem::IssueNamesUnknownNode { issue, id }),
            None => Some(Problem::MalformedFeatureLine {
                issue,
                value: value.to_owned(),
            }),
        }
    });
    problems
        .map(|problem| Finding {
            file: TreePath::new(config::FILE_NAME).unwrap_or_default(),
            line: None,
            problem,
        })
        .collect()
}

/// An issue as `work` and `stale` read it through `gh`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub reference: IssueRef,
    pub title: String,
    pub state: IssueState,
    /// A display label only, such as `Epic`.
    pub issue_type: Option<String>,
    pub parent: Option<IssueRef>,
    /// In GitHub's order.
    pub sub_issues: Vec<SubIssue>,
    pub body: String,
    /// RFC 3339, such as `2026-10-01T09:30:00Z`.
    pub created_at: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssueState {
    Open,
    Closed,
}

/// One entry in an issue's list of sub-issues.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubIssue {
    Issue(IssueRef),
    /// GitHub listed a sub-issue it couldn't return, so its reference is unknown.
    Unreadable(Unreadable),
}

/// Why `gh` couldn't read an issue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unreadable {
    /// The issue, or its repo, doesn't exist or can't be seen.
    NotFound,
    /// GitHub's message, such as for an issue or repo the token can't access.
    Failed(String),
    /// GitHub stopped listing an issue's sub-issues after `after` of them by repeating a page
    /// cursor, so the rest couldn't be read.
    Unlisted { after: usize },
}

/// An issue reference compared without case, as GitHub compares owner and repo names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IssueKey(String);

impl From<&IssueRef> for IssueKey {
    fn from(issue: &IssueRef) -> Self {
        Self(issue.to_string().to_ascii_lowercase())
    }
}

/// The issues `gh` read for a hierarchy, or why it couldn't read each one.
pub type Fetched = BTreeMap<IssueKey, Result<Issue, Unreadable>>;

/// Each issue's parent, for the issues above a node's work items. `None`: no parent, or none
/// `gh` could read.
pub type Parents = BTreeMap<IssueKey, Option<IssueRef>>;

/// The issues that nodes below a node list in their own `work`, each with the node that takes
/// it and its sub-issues.
pub type Takeovers<'a> = BTreeMap<IssueKey, &'a Entry>;

/// One issue in a node's work, with the sub-issues the node owns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkEntry<'a> {
    /// `None` for a sub-issue GitHub couldn't return.
    pub reference: Option<&'a IssueRef>,
    pub issue: Result<&'a Issue, &'a Unreadable>,
    /// The issue's parents, nearest first, up to the top.
    pub parents: Vec<&'a IssueRef>,
    /// How many of the issue's sub-issues are closed, and how many GitHub listed. `None` when a
    /// deeper node takes them, so they weren't read.
    pub done: Option<usize>,
    pub total: Option<usize>,
    /// A deeper node that lists this sub-issue in its own `work`, which takes the issue's
    /// sub-issues.
    pub listed_by: Option<&'a Entry>,
    pub sub_issues: Vec<WorkEntry<'a>>,
}

/// What `gh` read for a node's work: the issues and their sub-issues, and the parents above
/// the issues the node lists.
pub struct GitHubWork<'a> {
    pub fetched: &'a Fetched,
    pub parents: &'a Parents,
}

/// The issues the nodes below `entry` list in `work`, other than those `node` lists itself.
/// When two nodes list one issue, the shallower node takes it, and of two at one depth, the
/// first in outline order.
pub fn takeovers<'a>(tree: &'a Tree, entry: &'a Entry, node: &'a Node) -> Takeovers<'a> {
    let own: BTreeSet<IssueKey> = node.work.iter().map(IssueKey::from).collect();
    let mut below = tree.outline(&entry.location.folder(), None);
    below.retain(|(level, _)| *level > 0);
    // A stable sort keeps outline order within each level.
    below.sort_by_key(|(level, _)| *level);
    let mut taken = Takeovers::new();
    for (_, child) in below {
        let work = child.node.iter().flat_map(|node| node.work.iter());
        for issue in work.filter(|issue| !own.contains(&IssueKey::from(*issue))) {
            taken.entry(issue.into()).or_insert(child);
        }
    }
    taken
}

/// The issues `node` lists in `work`, each with its parents and its sub-issues at any depth. A
/// sub-issue in `takeovers` belongs to a deeper node, so its sub-issues are left out. An issue
/// the node lists that's also below another it lists shows once, below.
pub fn node_work<'a>(
    node: &'a Node,
    github: GitHubWork<'a>,
    takeovers: &'a Takeovers<'a>,
) -> Vec<WorkEntry<'a>> {
    let walk = Walk { github, takeovers };
    let entries: Vec<WorkEntry> = node
        .work
        .iter()
        .map(|issue| walk.entry(issue, walk.chain(issue), None, &[]))
        .collect();
    let nested = nested(&entries);
    let entries = entries.into_iter().zip(nested);
    entries
        .filter_map(|(item, nested)| (!nested).then_some(item))
        .collect()
}

/// Whether each top-level entry also shows below another. Two entries below each other form
/// a cycle, and then both show.
fn nested(entries: &[WorkEntry]) -> Vec<bool> {
    let keys: Vec<Option<IssueKey>> = entries
        .iter()
        .map(|item| item.reference.map(IssueKey::from))
        .collect();
    let below: Vec<BTreeSet<IssueKey>> = entries.iter().map(descendants).collect();
    let under = |i: usize, j: usize| keys[i].as_ref().is_some_and(|key| below[j].contains(key));
    (0..entries.len())
        .map(|i| (0..entries.len()).any(|j| j != i && under(i, j) && !under(j, i)))
        .collect()
}

/// The issues below `entry` in its output.
fn descendants(entry: &WorkEntry) -> BTreeSet<IssueKey> {
    let mut keys = BTreeSet::new();
    for sub in &entry.sub_issues {
        keys.extend(sub.reference.map(IssueKey::from));
        keys.extend(descendants(sub));
    }
    keys
}

struct Walk<'a> {
    github: GitHubWork<'a>,
    takeovers: &'a Takeovers<'a>,
}

impl<'a> Walk<'a> {
    // `above` guards against a cycle, which GitHub's sub-issues can't form.
    fn entry(
        &self,
        reference: &'a IssueRef,
        parents: Vec<&'a IssueRef>,
        listed_by: Option<&'a Entry>,
        above: &[IssueKey],
    ) -> WorkEntry<'a> {
        let key = IssueKey::from(reference);
        let issue = self.issue(reference);
        let subs = issue.map_or(&[][..], |issue| issue.sub_issues.as_slice());
        let expand = listed_by.is_none() && !above.contains(&key);
        let done = listed_by
            .is_none()
            .then(|| subs.iter().filter(|sub| self.closed(sub)).count());
        let listed = subs.iter().filter(|sub| !is_unlisted(sub)).count();
        let total = listed_by.is_none().then_some(listed);
        let mut path = above.to_vec();
        path.push(key);
        let chain = |parents: &[&'a IssueRef]| {
            let chain = std::iter::once(reference).chain(parents.iter().copied());
            chain.collect::<Vec<_>>()
        };
        let sub_issues = subs.iter().filter(|_| expand).map(|sub| match sub {
            SubIssue::Issue(sub) => {
                let listed_by = self.takeovers.get(&IssueKey::from(sub)).copied();
                self.entry(sub, chain(&parents), listed_by, &path)
            }
            SubIssue::Unreadable(why) => unreadable_sub_issue(why, chain(&parents)),
        });
        WorkEntry {
            reference: Some(reference),
            issue,
            parents: parents.clone(),
            done,
            total,
            listed_by,
            sub_issues: sub_issues.collect(),
        }
    }

    fn issue(&self, reference: &IssueRef) -> Result<&'a Issue, &'a Unreadable> {
        let fetched = self.github.fetched.get(&IssueKey::from(reference));
        // `fetch_hierarchy` reads every issue it's given, so a missing one isn't expected.
        fetched.map_or(Err(&Unreadable::NotFound), Result::as_ref)
    }

    fn closed(&self, sub: &SubIssue) -> bool {
        let SubIssue::Issue(sub) = sub else {
            return false;
        };
        let issue = self.issue(sub).ok();
        issue.is_some_and(|issue| issue.state == IssueState::Closed)
    }

    /// A work item's parents, nearest first, from its own `parent` up through `parents`.
    fn chain(&self, reference: &IssueRef) -> Vec<&'a IssueRef> {
        let mut seen = vec![IssueKey::from(reference)];
        let mut chain: Vec<&IssueRef> = Vec::new();
        let mut next = self.issue(reference).ok().and_then(|i| i.parent.as_ref());
        while let Some(parent) = next {
            let key = IssueKey::from(parent);
            if seen.contains(&key) {
                break;
            }
            chain.push(parent);
            next = self.github.parents.get(&key).and_then(Option::as_ref);
            seen.push(key);
        }
        chain
    }
}

/// The mark for sub-issues GitHub stopped listing, which isn't a sub-issue to count.
fn is_unlisted(sub: &SubIssue) -> bool {
    matches!(sub, SubIssue::Unreadable(Unreadable::Unlisted { .. }))
}

fn unreadable_sub_issue<'a>(why: &'a Unreadable, parents: Vec<&'a IssueRef>) -> WorkEntry<'a> {
    WorkEntry {
        reference: None,
        issue: Err(why),
        parents,
        done: None,
        total: None,
        listed_by: None,
        sub_issues: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feature(body: &str) -> Option<String> {
        IssueLinks::parse(body).feature.map(|id| id.to_string())
    }

    #[test]
    fn reads_feature_and_spec_ref_lines() {
        let links = IssueLinks::parse("Intro.\n\nfeature: fd-7c2e41\nspec-ref: 4f1c2ab\n");
        assert_eq!(
            links.feature.map(|id| id.to_string()),
            Some("fd-7c2e41".into())
        );
        assert_eq!(links.spec_ref, Some("4f1c2ab".into()));
    }

    #[test]
    fn reads_nothing_from_a_body_without_link_lines() {
        assert_eq!(IssueLinks::parse(""), IssueLinks::default());
        assert_eq!(IssueLinks::parse("Fix search.\n"), IssueLinks::default());
    }

    #[test]
    fn counts_only_lines_that_start_with_the_key() {
        for body in [
            " feature: fd-7c2e41",
            "- feature: fd-7c2e41",
            "See feature: fd-7c2e41",
            "Feature: fd-7c2e41",
            "features: fd-7c2e41",
        ] {
            assert_eq!(feature(body), None, "{body}");
        }
    }

    #[test]
    fn ignores_lines_in_code_blocks() {
        for body in [
            "```\nfeature: fd-7c2e41\n```\n",
            "~~~yaml\nfeature: fd-7c2e41\n~~~\n",
            "````\n```\nfeature: fd-7c2e41\n```\n````\n",
            "```\nnever closed\nfeature: fd-7c2e41\n",
        ] {
            assert_eq!(feature(body), None, "{body}");
        }
        assert_eq!(
            feature("```\nfeature: fd-000001\n```\nfeature: fd-7c2e41\n"),
            Some("fd-7c2e41".into())
        );
    }

    #[test]
    fn reads_bodies_with_windows_line_endings() {
        let links = IssueLinks::parse("Intro.\r\n\r\nfeature: fd-7c2e41\r\nspec-ref: 4f1c2ab\r\n");
        assert_eq!(
            links.feature.map(|id| id.to_string()),
            Some("fd-7c2e41".into())
        );
        assert_eq!(links.spec_ref, Some("4f1c2ab".into()));
    }

    #[test]
    fn the_first_line_with_a_key_decides() {
        assert_eq!(
            feature("feature: fd-7c2e41\nfeature: fd-000001\n"),
            Some("fd-7c2e41".into())
        );
        assert_eq!(feature("feature: fd-xyz\nfeature: fd-7c2e41\n"), None);
    }

    #[test]
    fn gives_no_link_for_a_value_that_isnt_one() {
        assert_eq!(feature("feature: fd-7C2E41"), None);
        assert_eq!(feature("feature: fd-7c2e41 search"), None);
        assert_eq!(feature("feature:"), None);
        assert_eq!(IssueLinks::parse("spec-ref:   \n").spec_ref, None);
    }
}
