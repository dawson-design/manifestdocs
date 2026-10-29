//! `manifestdocs work`: the issues a node lists in `work`, with their sub-issues, read from
//! GitHub through `gh`.

use std::env;
use std::fmt::Write as _;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use manifestdocs_core::refs::{IssueRef, RepoRef};
use manifestdocs_core::tree::{Entry, Tree};
use manifestdocs_core::work::{
    Fetched, GitHubWork, IssueKey, IssueState, Unreadable, WorkEntry, node_work, takeovers,
};

use crate::files::{self, ByDefault};
use crate::gh;
use crate::json;
use crate::locate::locate;
use crate::report::write_stdout;
use crate::show::{Paths, problems, select_node};

/// Reads the docs as `context` does, from the default branch from another repo, since it
/// feeds agents planning work. Exits 1 when the node's frontmatter has errors, which it
/// prints, or when `gh` can't read an issue, which it marks.
pub fn run(arg: &str, json: bool, reference: Option<&str>) -> Result<ExitCode> {
    let cwd = env::current_dir().context("can't read the current folder")?;
    let docs = files::open(locate(&cwd)?, reference, ByDefault::DefaultBranch, &cwd)?;
    let roots = &docs.roots;
    let (tree, snapshot) = files::tree_and_snapshot(&docs, &cwd)?;
    let Some(github) = &tree.config.github else {
        bail!("`work` reads issues from GitHub, and manifestdocs.yaml has no `github` block");
    };
    let paths = Paths {
        tree_root: &roots.tree_root,
        cwd: &cwd,
    };
    let entry = select_node(&tree, &snapshot, arg, &paths)?;
    let node = match &entry.node {
        Ok(node) => node,
        Err(findings) => {
            match json {
                true => json::work(entry, None, &roots.tree_root, &cwd)?,
                false => write_stdout(&problems(findings, &paths))?,
            }
            return Ok(ExitCode::from(1));
        }
    };
    let takeovers = takeovers(&tree, entry, node);
    let fetched = gh::fetch_hierarchy(&github.host, &node.work, &takeovers)?;
    let parents = gh::fetch_parents(&github.host, &top_parents(&node.work, &fetched))?;
    let github = GitHubWork {
        fetched: &fetched,
        parents: &parents,
    };
    let work = node_work(node, github, &takeovers);
    match json {
        true => json::work(entry, Some(&work), &roots.tree_root, &cwd)?,
        false => write_stdout(&render(&tree, entry, &work))?,
    }
    Ok(match work.iter().any(unreadable) {
        true => ExitCode::from(1),
        false => ExitCode::SUCCESS,
    })
}

/// The parents of the work items `gh` could read, where their chains start.
fn top_parents(work: &[IssueRef], fetched: &Fetched) -> Vec<IssueRef> {
    let issues = work
        .iter()
        .filter_map(|issue| fetched.get(&IssueKey::from(issue)));
    let issues = issues.filter_map(|issue| issue.as_ref().ok());
    issues.filter_map(|issue| issue.parent.clone()).collect()
}

fn unreadable(entry: &WorkEntry) -> bool {
    entry.issue.is_err() || entry.sub_issues.iter().any(unreadable)
}

/// The node's heading, then one line per issue, with each sub-issue indented under its parent.
fn render(tree: &Tree, entry: &Entry, work: &[WorkEntry]) -> String {
    let mut out = format!("{}\n", tree.heading(entry));
    if work.is_empty() {
        out.push_str("No work items.\n");
    }
    for item in work {
        push_line(tree, item, None, &mut out);
    }
    out
}

/// A top-level line names its issue in full and has no `above`. A sub-issue in the same repo
/// as the issue above it is named by number only.
fn push_line(tree: &Tree, item: &WorkEntry, above: Option<(&RepoRef, usize)>, out: &mut String) {
    let depth = above.map_or(0, |(_, depth)| depth + 1);
    let indent = "  ".repeat(depth);
    let repo_above = above.map(|(repo, _)| repo);
    let reference = item
        .reference
        .map_or("unknown sub-issue".to_owned(), |issue| {
            short(issue, repo_above)
        });
    let (Some(own), Ok(issue)) = (item.reference, item.issue) else {
        let why = item.issue.err().map_or(String::new(), reason);
        match item.issue {
            Err(Unreadable::Unlisted { .. }) => writeln!(out, "{indent}- {why}"),
            _ => writeln!(out, "{indent}- {reference}: {why}"),
        }
        .unwrap_or_default();
        return;
    };
    let mut notes: Vec<String> = issue.issue_type.iter().cloned().collect();
    notes.push(state(issue.state).to_owned());
    if let (Some(done), Some(total @ 1..)) = (item.done, item.total) {
        notes.push(format!("{done} of {total} done"));
    }
    if above.is_none() && !item.parents.is_empty() {
        let chain = item.parents.iter().rev();
        let chain: Vec<String> = chain
            .map(|parent| short(parent, Some(own.repo())))
            .collect();
        notes.push(format!("sub-issue of {}", chain.join(" > ")));
    }
    let title = &issue.title;
    let _ = write!(out, "{indent}- {reference} {title} ({})", notes.join(", "));
    if let Some(node) = item.listed_by {
        let _ = write!(out, ", listed by {} ({})", tree.name(node), node.id);
    }
    out.push('\n');
    for sub in &item.sub_issues {
        push_line(tree, sub, Some((own.repo(), depth)), out);
    }
}

/// `#number` when the issue is in `repo`, and `owner/name#number` otherwise.
fn short(issue: &IssueRef, repo: Option<&RepoRef>) -> String {
    let same = |repo: &RepoRef| {
        let name = repo.to_string();
        name.eq_ignore_ascii_case(&issue.repo().to_string())
    };
    match repo.is_some_and(same) {
        true => format!("#{}", issue.number()),
        false => issue.to_string(),
    }
}

/// Why an issue's line has no title, in text and in JSON.
pub fn reason(unreadable: &Unreadable) -> String {
    match unreadable {
        Unreadable::NotFound => "not an issue you can read on GitHub".to_owned(),
        Unreadable::Failed(message) => format!("can't read it on GitHub: {message}"),
        Unreadable::Unlisted { after } => {
            format!("sub-issues past {after} couldn't be read, since GitHub repeated a page cursor")
        }
    }
}

pub fn state(state: IssueState) -> &'static str {
    match state {
        IssueState::Open => "open",
        IssueState::Closed => "closed",
    }
}
