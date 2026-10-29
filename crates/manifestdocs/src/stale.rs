//! `manifestdocs stale`: the open issues whose node changed after the docs version they were
//! planned against, read from GitHub through `gh` and from the docs clone's history. It only
//! reports.

use std::env;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use manifestdocs_core::check::Snapshot;
use manifestdocs_core::config::TreeConfig;
use manifestdocs_core::refs::{Host, IssueRef};
use manifestdocs_core::stale::{
    Baseline, Linked, Watched, is_commit_id, issues_to_read, linked_issues, watched,
};
use manifestdocs_core::tree::{Entry, Tree};
use manifestdocs_core::work::{Issue, IssueBody, Takeovers, Unreadable, work_links};
use serde::Serialize;

use crate::files::{self, ByDefault, Docs, DocsSource};
use crate::gh;
use crate::git::{self, Commit, FileChange, Lines, PathSet};
use crate::json;
use crate::locate::locate;
use crate::report::{plural, relative, write_stdout};
use crate::work::reason;

/// Reads the docs as `work` does: the default branch from another repo, and the checked-out
/// commit inside the docs clone. Exits 1 when an issue is stale or can't be checked.
pub fn run(json: bool) -> Result<ExitCode> {
    let cwd = env::current_dir().context("can't read the current folder")?;
    let docs = files::open(locate(&cwd)?, None, ByDefault::DefaultBranch, &cwd)?;
    let (tree, snapshot) = files::tree_and_snapshot(&docs, &cwd)?;
    let Some(github) = &tree.config.github else {
        bail!("`stale` reads issues from GitHub, and manifestdocs.yaml has no `github` block");
    };
    let named = search(&github.host, &tree.config, &snapshot)?;
    let to_read = issues_to_read(&tree, &named);
    let fetched = gh::fetch_hierarchy(&github.host, &to_read, &Takeovers::new())?;
    let linked = linked_issues(&tree, &fetched, &named);
    let history = History::open(&docs, &cwd, linked.iter().any(needs_default_branch))?;
    let context = Context {
        tree: &tree,
        snapshot: &snapshot,
        docs: &docs,
        cwd: &cwd,
    };
    let checked: Vec<Checked> = linked
        .iter()
        .map(|linked| history.check(&context, linked))
        .collect::<Result<_>>()?;
    match json {
        true => json::data("stale", data(&checked, &context))?,
        false => write_stdout(&render(&checked, &context))?,
    }
    let failed = checked.iter().any(|item| item.outcome.is_reported());
    Ok(if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

/// The issues in the docs repo and the nodes' repos whose bodies have a `feature:` line.
fn search(host: &Host, config: &TreeConfig, snapshot: &Snapshot) -> Result<Vec<IssueBody>> {
    let Some(links) = work_links(config, snapshot) else {
        return Ok(Vec::new());
    };
    let mut named = Vec::new();
    for repo in &links.searched_repos {
        let found = gh::search_issues(host, repo)?;
        if found.total > found.read {
            eprintln!(
                "note: the search of `{repo}` for `feature:` lines stopped at {} of {} issues, since GitHub search returns at most 1,000",
                found.read, found.total
            );
        }
        named.extend(found.issues);
    }
    Ok(named)
}

fn needs_default_branch(linked: &Linked) -> bool {
    matches!(
        linked,
        Linked::Open {
            baseline: Baseline::Created(_),
            ..
        }
    )
}

/// What the report needs from the tree that was read.
struct Context<'a> {
    tree: &'a Tree,
    snapshot: &'a Snapshot,
    docs: &'a Docs,
    cwd: &'a Path,
}

impl Context<'_> {
    fn watched(&self, node: &Entry) -> PathSet {
        let snapshot = self.snapshot;
        let Watched { folder, excluded } =
            watched(self.tree, node, &snapshot.tree_root, &snapshot.paths);
        PathSet {
            folder: folder.to_string(),
            excluded: excluded.iter().map(ToString::to_string).collect(),
        }
    }

    fn node_file(&self, node: &Entry) -> String {
        relative(
            &self.docs.roots.tree_root.join(node.file.as_str()),
            self.cwd,
        )
    }

    /// A path from the repo root, as the current folder names it.
    fn repo_file(&self, path: &str) -> String {
        relative(&self.docs.roots.repo_root.join(path), self.cwd)
    }
}

/// The docs clone's history: the commit read, and the default branch when an issue without
/// a `spec-ref:` needs it.
struct History {
    clone: PathBuf,
    head: Commit,
    default_branch: Option<Commit>,
}

/// One linked issue, and what comparing it found.
struct Checked<'a> {
    node: &'a Entry,
    reference: Option<&'a IssueRef>,
    issue: Option<&'a Issue>,
    outcome: Outcome<'a>,
}

enum Outcome<'a> {
    Current,
    Stale(Change<'a>),
    BadSpecRef(&'a str),
    Unread(&'a Unreadable),
}

impl Outcome<'_> {
    fn is_reported(&self) -> bool {
        !matches!(self, Self::Current)
    }
}

/// The node's latest change after the version an issue was planned against, `since`, which
/// is `None` when no docs commit is older than the issue.
struct Change<'a> {
    baseline: &'a Baseline,
    since: Option<Commit>,
    latest: Commit,
    files: Vec<FileChange>,
}

impl History {
    fn open(docs: &Docs, cwd: &Path, needs_default_branch: bool) -> Result<Self> {
        let clone = docs.roots.repo_root.clone();
        let shown = relative(&clone, cwd);
        if !clone.join(".git").exists() {
            let tree = relative(&docs.roots.tree_root, cwd);
            bail!("`stale` compares the docs' history, and the tree `{tree}` isn't in a git repo");
        }
        let head = match &docs.source {
            DocsSource::Commit { reference, .. } => reference.as_str(),
            DocsSource::WorkingTree => "HEAD",
        };
        let resolve = |reference: &str| {
            git::resolve(&clone, reference)
                .ok_or_else(|| anyhow!("no commit `{reference}` in the docs clone `{shown}`"))
        };
        let head = resolve(head)?;
        let default_branch = match needs_default_branch {
            true => Some(resolve(&files::default_branch_of(&clone, &shown)?)?),
            false => None,
        };
        Ok(Self {
            clone,
            head,
            default_branch,
        })
    }

    fn check<'a>(&self, context: &Context, linked: &'a Linked<'a>) -> Result<Checked<'a>> {
        Ok(match linked {
            Linked::Unread {
                node,
                reference,
                why,
            } => Checked {
                node,
                reference: *reference,
                issue: None,
                outcome: Outcome::Unread(why),
            },
            Linked::Open {
                node,
                issue,
                baseline,
            } => Checked {
                node,
                reference: Some(&issue.reference),
                issue: Some(issue),
                outcome: self.compare(baseline, &context.watched(node))?,
            },
        })
    }

    fn compare<'a>(&self, baseline: &'a Baseline, paths: &PathSet) -> Result<Outcome<'a>> {
        let since = match baseline {
            Baseline::SpecRef(spec_ref) => match self.spec_ref(spec_ref) {
                Some(commit) => Some(commit),
                None => return Ok(Outcome::BadSpecRef(spec_ref)),
            },
            Baseline::Created(time) => {
                let branch = self.default_branch.as_ref();
                let branch = branch.context("the docs' default branch wasn't read")?;
                git::commit_before(&self.clone, &branch.id, time)?
            }
        };
        let base = since.as_ref().map(|since| since.id.as_str());
        let Some(latest) = git::latest_change(&self.clone, base, &self.head.id, paths)? else {
            return Ok(Outcome::Current);
        };
        let files = git::diff_summary(&self.clone, base, &latest.id, paths)?;
        Ok(Outcome::Stale(Change {
            baseline,
            since,
            latest,
            files,
        }))
    }

    fn spec_ref(&self, spec_ref: &str) -> Option<Commit> {
        let id = is_commit_id(spec_ref).then_some(spec_ref)?;
        git::resolve(&self.clone, id)
    }
}

fn bad_spec_ref(spec_ref: &str) -> String {
    format!("its spec-ref `{spec_ref}` isn't a commit in the docs clone")
}

/// A block for each stale issue or issue that couldn't be checked, then a summary line.
fn render(checked: &[Checked], context: &Context) -> String {
    let mut out = String::new();
    for item in checked.iter().filter(|item| item.outcome.is_reported()) {
        push_block(item, context, &mut out);
        out.push('\n');
    }
    out.push_str(&summary(checked));
    out
}

fn push_block(item: &Checked, context: &Context, out: &mut String) {
    let reference = item
        .reference
        .map_or("unknown sub-issue".into(), ToString::to_string);
    let title = item
        .issue
        .map_or(String::new(), |issue| format!(" {}", issue.title));
    let heading = context.tree.heading(item.node);
    let _ = writeln!(out, "{reference}{title}");
    let _ = writeln!(out, "  {heading}  {}", context.node_file(item.node));
    let why = match &item.outcome {
        Outcome::Current => return,
        Outcome::Stale(change) => return push_change(change, context, out),
        Outcome::BadSpecRef(spec_ref) => format!("can't check it: {}", bad_spec_ref(spec_ref)),
        Outcome::Unread(why) => reason(why),
    };
    let _ = writeln!(out, "  {why}");
}

fn push_change(change: &Change, context: &Context, out: &mut String) {
    let latest = &change.latest;
    let changed = format!("changed in {} ({})", latest.short, latest.date);
    let line = match (change.baseline, &change.since) {
        (Baseline::SpecRef(_), Some(since)) => {
            format!(
                "{changed} after its spec-ref {} ({})",
                since.short, since.date
            )
        }
        (Baseline::Created(_), Some(since)) => format!(
            "{changed} after {} ({}), the docs when it was created",
            since.short, since.date
        ),
        (_, None) => format!("{changed}; the issue is older than every docs commit"),
    };
    let _ = writeln!(out, "  {line}");
    for file in &change.files {
        let lines = match file.lines {
            Lines::Text { added, removed } => format!("+{added} -{removed}"),
            Lines::Binary => "binary".to_owned(),
        };
        let _ = writeln!(out, "    {} ({lines})", context.repo_file(&file.path));
    }
}

/// Such as `3 open issues: 1 stale, 1 not checked`.
fn summary(checked: &[Checked]) -> String {
    let open = checked.iter().filter(|item| item.issue.is_some()).count();
    let count = |stale: bool| {
        let reported = checked.iter().filter(|item| item.outcome.is_reported());
        let stale_or_not =
            reported.filter(|item| matches!(item.outcome, Outcome::Stale(_)) == stale);
        stale_or_not.count()
    };
    let mut parts = vec![match count(true) {
        0 => "none stale".to_owned(),
        stale => format!("{stale} stale"),
    }];
    if let not_checked @ 1.. = count(false) {
        parts.push(format!("{not_checked} not checked"));
    }
    format!("{}: {}\n", plural(open, "open issue"), parts.join(", "))
}

#[derive(Serialize)]
struct StaleData<'a> {
    open: usize,
    stale: Vec<StaleIssue<'a>>,
    not_checked: Vec<NotChecked<'a>>,
}

#[derive(Serialize)]
struct StaleIssue<'a> {
    issue: String,
    title: &'a str,
    node: &'a str,
    file: String,
    spec_ref: Option<&'a str>,
    created_at: &'a str,
    since: Option<JsonCommit<'a>>,
    changed_in: JsonCommit<'a>,
    changes: Vec<JsonChange>,
}

#[derive(Serialize)]
struct JsonCommit<'a> {
    commit: &'a str,
    date: &'a str,
}

/// `added` and `removed` are `None` for a binary file.
#[derive(Serialize)]
struct JsonChange {
    path: String,
    added: Option<u64>,
    removed: Option<u64>,
}

/// `issue` is `None` for a sub-issue GitHub listed but couldn't return, and `title` when
/// `gh` couldn't read the issue.
#[derive(Serialize)]
struct NotChecked<'a> {
    issue: Option<String>,
    title: Option<&'a str>,
    node: &'a str,
    file: String,
    error: String,
}

fn data<'a>(checked: &'a [Checked<'a>], context: &Context) -> StaleData<'a> {
    let mut data = StaleData {
        open: checked.iter().filter(|item| item.issue.is_some()).count(),
        stale: Vec::new(),
        not_checked: Vec::new(),
    };
    for item in checked {
        let not_checked = |error| not_checked(item, error, context);
        match &item.outcome {
            Outcome::Current => {}
            Outcome::Stale(change) => data.stale.extend(stale_issue(item, change, context)),
            Outcome::BadSpecRef(spec_ref) => {
                data.not_checked.push(not_checked(bad_spec_ref(spec_ref)))
            }
            Outcome::Unread(why) => data.not_checked.push(not_checked(reason(why))),
        }
    }
    data
}

fn stale_issue<'a>(
    item: &Checked<'a>,
    change: &'a Change,
    context: &Context,
) -> Option<StaleIssue<'a>> {
    let issue = item.issue?;
    let commit = |commit: &'a Commit| JsonCommit {
        commit: &commit.id,
        date: &commit.date,
    };
    Some(StaleIssue {
        issue: issue.reference.to_string(),
        title: &issue.title,
        node: item.node.id.as_str(),
        file: context.node_file(item.node),
        spec_ref: match change.baseline {
            Baseline::SpecRef(spec_ref) => Some(spec_ref),
            Baseline::Created(_) => None,
        },
        created_at: &issue.created_at,
        since: change.since.as_ref().map(commit),
        changed_in: commit(&change.latest),
        changes: change
            .files
            .iter()
            .map(|file| json_change(file, context))
            .collect(),
    })
}

fn json_change(file: &FileChange, context: &Context) -> JsonChange {
    let (added, removed) = match file.lines {
        Lines::Text { added, removed } => (Some(added), Some(removed)),
        Lines::Binary => (None, None),
    };
    JsonChange {
        path: context.repo_file(&file.path),
        added,
        removed,
    }
}

fn not_checked<'a>(item: &Checked<'a>, error: String, context: &Context) -> NotChecked<'a> {
    NotChecked {
        issue: item.reference.map(ToString::to_string),
        title: item.issue.map(|issue| issue.title.as_str()),
        node: item.node.id.as_str(),
        file: context.node_file(item.node),
        error,
    }
}
