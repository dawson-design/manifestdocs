//! Links to work: the node links in issue bodies, and `check`'s rules for the issues a
//! tree references and the issues that name its nodes.

use std::collections::{BTreeMap, BTreeSet};
use std::iter;
use std::ops::Range;

use pulldown_cmark::{Event, Parser, Tag};

use crate::check::{Report, Snapshot, read_node};
use crate::config::{self, TreeConfig};
use crate::finding::{Field, Finding, Problem};
use crate::id::NodeId;
use crate::layout::layout;
use crate::node::ParsedNode;
use crate::paths::TreePath;
use crate::refs::{IssueRef, RepoRef};

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
