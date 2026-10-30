//! `manifestdocs stale`: the open issues whose node changed after the docs version they were
//! planned against, read from GitHub through `gh` and from the docs clone's history. It only
//! reports.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use manifestdocs_core::check::Snapshot;
use manifestdocs_core::config::TreeConfig;
use manifestdocs_core::paths::{RepoPath, TreePath};
use manifestdocs_core::refs::{Host, IssueRef};
use manifestdocs_core::stale::{
    Baseline, Linked, Version, Watched, is_commit_id, issues_to_read, linked_issues,
    only_work_changed, watched,
};
use manifestdocs_core::tree::{Entry, Tree, node_files_to_read, read_tree};
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
    let mut history = History::open(&docs, &cwd)?;
    let named = search(&github.host, &tree.config, &snapshot)?;
    let to_read = issues_to_read(&tree, &named);
    let fields = gh::Fields::WithText;
    let fetched = gh::fetch_hierarchy(&github.host, &to_read, &Takeovers::new(), fields)?;
    let linked = linked_issues(&tree, &fetched, &named);
    if linked.iter().any(needs_default_branch) {
        history.read_default_branch()?;
    }
    let context = Context {
        tree: &tree,
        snapshot: &snapshot,
        docs: &docs,
        cwd: &cwd,
    };
    let checked = history.check_all(&context, &linked)?;
    match json {
        true => json::data("stale", data(&checked, &context))?,
        false => write_stdout(&render(&checked, &context))?,
    }
    let failed = checked.iter().any(|item| item.outcome.is_reported());
    Ok(ExitCode::from(u8::from(failed)))
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
    /// The files `node` owns, less those a deeper node owned at `baseline`.
    fn watched(&self, node: &Entry, baseline: Option<&(Tree, Snapshot)>) -> PathSet {
        let head = Version {
            tree: self.tree,
            snapshot: self.snapshot,
        };
        let baseline = baseline.map(|(tree, snapshot)| Version { tree, snapshot });
        let Watched { folder, excluded } = watched(node, head, baseline);
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

/// The docs clone's history: the commit read, the default branch when an issue without a
/// `spec-ref:` needs it, and what's been read, since many issues share a node and a baseline.
struct History {
    clone: PathBuf,
    /// The clone as the current folder names it, for messages.
    shown: String,
    /// The version read, as messages name it: `HEAD` inside the docs clone.
    head_name: String,
    head: Commit,
    default_branch: Option<Commit>,
    /// The default branch's commit before each issue creation time.
    before: BTreeMap<String, Option<Commit>>,
    /// The tree at each baseline commit.
    baselines: BTreeMap<String, (Tree, Snapshot)>,
    /// The files in each commit a node last changed in.
    listings: BTreeMap<String, BTreeSet<RepoPath>>,
    /// Each node file's history after each baseline commit.
    compared: BTreeMap<(TreePath, Option<String>), Compared>,
}

/// A node's history after a baseline.
#[derive(Clone)]
enum Compared {
    Current,
    Changed {
        latest: Commit,
        files: Vec<FileChange>,
    },
    /// Why the diff couldn't be read, such as a partial clone missing a file.
    NoDiff(String),
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
    /// Why the issue couldn't be compared.
    NotChecked(String),
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
    /// Fails when the tree isn't in git, or the clone is shallow, since a shallow clone's
    /// history stops early and every issue would look older than the docs.
    fn open(docs: &Docs, cwd: &Path) -> Result<Self> {
        let clone = docs.roots.repo_root.clone();
        let shown = relative(&clone, cwd);
        has_full_history(docs, cwd, &shown)?;
        let head_name = match &docs.source {
            DocsSource::Commit { reference, .. } => reference.clone(),
            DocsSource::WorkingTree => "HEAD".to_owned(),
        };
        let head = resolve(&clone, &shown, &head_name)?;
        Ok(Self {
            clone,
            shown,
            head_name,
            head,
            default_branch: None,
            before: BTreeMap::new(),
            baselines: BTreeMap::new(),
            listings: BTreeMap::new(),
            compared: BTreeMap::new(),
        })
    }

    fn read_default_branch(&mut self) -> Result<()> {
        let branch = files::default_branch_of(&self.clone, &self.shown)?;
        self.default_branch = Some(resolve(&self.clone, &self.shown, &branch)?);
        Ok(())
    }

    fn check_all<'a>(
        &mut self,
        context: &Context,
        linked: &'a [Linked<'a>],
    ) -> Result<Vec<Checked<'a>>> {
        linked
            .iter()
            .map(|linked| self.check(context, linked))
            .collect()
    }

    fn check<'a>(&mut self, context: &Context, linked: &'a Linked<'a>) -> Result<Checked<'a>> {
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
                outcome: self.compare(context, node, baseline)?,
            },
        })
    }

    fn compare<'a>(
        &mut self,
        context: &Context,
        node: &Entry,
        baseline: &'a Baseline,
    ) -> Result<Outcome<'a>> {
        let since = match self.since(baseline)? {
            Ok(since) => since,
            Err(why) => return Ok(Outcome::NotChecked(why)),
        };
        Ok(match self.compared(context, node, since.as_ref()) {
            Compared::Current => Outcome::Current,
            Compared::NoDiff(why) => Outcome::NotChecked(why),
            Compared::Changed { latest, files } => Outcome::Stale(Change {
                baseline,
                since,
                latest,
                files,
            }),
        })
    }

    /// The commit an issue was planned against, `None` when no docs commit is older than the
    /// issue, or why its `spec-ref` can't be used.
    fn since(&mut self, baseline: &Baseline) -> Result<Result<Option<Commit>, String>> {
        match baseline {
            Baseline::SpecRef(spec_ref) => Ok(self.spec_ref(spec_ref)?.map(Some)),
            Baseline::Created(time) => Ok(Ok(self.before(time)?)),
        }
    }

    /// A `spec-ref` outside the history read, such as on a branch that was squash-merged or
    /// never merged, would look stale forever, so it isn't used.
    fn spec_ref(&self, spec_ref: &str) -> Result<Result<Commit, String>> {
        let commit = is_commit_id(spec_ref).then(|| git::resolve(&self.clone, spec_ref));
        let Some(commit) = commit.flatten() else {
            let why = format!("its spec-ref `{spec_ref}` isn't a commit in the docs clone");
            return Ok(Err(why));
        };
        if !git::is_ancestor(&self.clone, &commit.id, &self.head.id)? {
            let head = &self.head_name;
            let why = format!("its spec-ref `{spec_ref}` isn't in the history of `{head}`");
            return Ok(Err(why));
        }
        Ok(Ok(commit))
    }

    fn before(&mut self, time: &str) -> Result<Option<Commit>> {
        if let Some(found) = self.before.get(time) {
            return Ok(found.clone());
        }
        let branch = self.default_branch.as_ref();
        let branch = branch.context("the docs' default branch wasn't read")?;
        let found = git::commit_before(&self.clone, &branch.id, time)?;
        self.before.insert(time.to_owned(), found.clone());
        Ok(found)
    }

    /// A history that can't be read, such as when a partial clone lacks a commit's folders or
    /// files, leaves only this issue unchecked, since the tool never fetches.
    fn compared(&mut self, context: &Context, node: &Entry, since: Option<&Commit>) -> Compared {
        let key = (node.file.clone(), since.map(|since| since.id.clone()));
        if let Some(found) = self.compared.get(&key) {
            return found.clone();
        }
        let found = self.read_history(context, node, since);
        let found = found.unwrap_or_else(|error| {
            let head = &self.head.short;
            Compared::NoDiff(match since {
                Some(since) => format!(
                    "the docs clone can't compare {} with {head}: {error:#}",
                    since.short
                ),
                None => format!("the docs clone can't read its history up to {head}: {error:#}"),
            })
        });
        self.compared.insert(key, found.clone());
        found
    }

    fn read_history(
        &mut self,
        context: &Context,
        node: &Entry,
        since: Option<&Commit>,
    ) -> Result<Compared> {
        let baseline = match since {
            Some(since) => Some(self.baseline(context, &since.id)?),
            None => None,
        };
        let node_file = context.snapshot.tree_root.join(node.file.as_str());
        let at_baseline = baseline.is_some_and(|(_, snapshot)| {
            node_file
                .as_ref()
                .is_some_and(|file| snapshot.paths.contains(file))
        });
        let paths = context.watched(node, baseline);
        let found = self.diff(since, &paths)?;
        if let (Some(since), Some(node_file), true, Compared::Changed { latest, files }) =
            (since, node_file, at_baseline, &found)
            && self.work_only(&node_file, since, latest, files)?
        {
            return Ok(Compared::Current);
        }
        Ok(found)
    }

    /// Whether the node file, which is at the baseline, is the only file that changed, and
    /// only in its `work` list. A node file that's gone at `latest` is a change.
    fn work_only(
        &mut self,
        node_file: &RepoPath,
        since: &Commit,
        latest: &Commit,
        files: &[FileChange],
    ) -> Result<bool> {
        let [file] = files else {
            return Ok(false);
        };
        if file.path != node_file.as_str() || !self.listing(&latest.id)?.contains(node_file) {
            return Ok(false);
        }
        let before = git::text_at(&self.clone, &since.id, &file.path)?;
        let after = git::text_at(&self.clone, &latest.id, &file.path)?;
        Ok(only_work_changed(&before, &after))
    }

    /// The tree at commit `id`, its nodes read from their files there with the config read
    /// now, so the nodes there are known by their `id` as they are now.
    fn baseline(&mut self, context: &Context, id: &str) -> Result<&(Tree, Snapshot)> {
        if !self.baselines.contains_key(id) {
            let config = &context.tree.config;
            let tree_root = &context.snapshot.tree_root;
            let version = read_version(&self.clone, config, tree_root, id)?;
            self.baselines.insert(id.to_owned(), version);
        }
        Ok(&self.baselines[id])
    }

    fn listing(&mut self, id: &str) -> Result<&BTreeSet<RepoPath>> {
        if !self.listings.contains_key(id) {
            let files = git::files_at(&self.clone, id)?.into_iter();
            let files = files.filter_map(|(path, _)| RepoPath::new(&path)).collect();
            self.listings.insert(id.to_owned(), files);
        }
        Ok(&self.listings[id])
    }

    fn diff(&self, since: Option<&Commit>, paths: &PathSet) -> Result<Compared> {
        let base = since.map(|since| since.id.as_str());
        let Some(latest) = git::latest_change(&self.clone, base, &self.head.id, paths)? else {
            return Ok(Compared::Current);
        };
        let files = git::diff_summary(&self.clone, base, &latest.id, paths)?;
        Ok(Compared::Changed { latest, files })
    }
}

/// Fails when the tree isn't in git, or its clone, which `shown` names, is shallow.
fn has_full_history(docs: &Docs, cwd: &Path, shown: &str) -> Result<()> {
    let clone = &docs.roots.repo_root;
    if !clone.join(".git").exists() {
        let tree = relative(&docs.roots.tree_root, cwd);
        bail!("`stale` compares the docs' history, and the tree `{tree}` isn't in a git repo");
    }
    if git::is_shallow(clone)? {
        bail!(
            "the docs clone `{shown}` is shallow, so `stale` can't read its history; fetch \
             all of it, such as with `git -C {shown} fetch --unshallow`, or `fetch-depth: 0` \
             for actions/checkout"
        );
    }
    Ok(())
}

/// The files in commit `id`, with the tree's node files read, and the tree they make.
fn read_version(
    clone: &Path,
    config: &TreeConfig,
    tree_root: &RepoPath,
    id: &str,
) -> Result<(Tree, Snapshot)> {
    let files = git::files_at(clone, id)?.into_iter();
    let blobs: BTreeMap<RepoPath, String> = files
        .filter_map(|(path, blob)| Some((RepoPath::new(&path)?, blob)))
        .collect();
    let mut snapshot = Snapshot {
        tree_root: tree_root.clone(),
        paths: blobs.keys().cloned().collect(),
        markdown: BTreeMap::new(),
    };
    let mut reader = git::Blobs::open(clone)?;
    for path in node_files_to_read(config, tree_root, &snapshot.paths) {
        let blob = blobs.get(&path).context("a listed file has no blob")?;
        let text = String::from_utf8_lossy(&reader.read(blob)?).into_owned();
        snapshot.markdown.insert(path, text);
    }
    Ok((read_tree(config, &snapshot), snapshot))
}

fn resolve(clone: &Path, shown: &str, reference: &str) -> Result<Commit> {
    git::resolve(clone, reference)
        .ok_or_else(|| anyhow!("no commit `{reference}` in the docs clone `{shown}`"))
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
        Outcome::NotChecked(why) => format!("can't check it: {why}"),
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
            Outcome::NotChecked(why) => data.not_checked.push(not_checked(why.clone())),
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
    let text = issue.text.as_ref()?;
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
        created_at: &text.created_at,
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
