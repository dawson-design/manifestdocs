//! Reads the tree's config, the repo's file list and the tree's Markdown, from the working
//! tree or from a commit in the docs clone.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use ignore::{DirEntry, WalkBuilder};
use manifestdocs_core::check::{Snapshot, markdown_to_read, outside_anchor_targets};
use manifestdocs_core::config::{self, TreeConfig};
use manifestdocs_core::paths::RepoPath;
use manifestdocs_core::record::records_to_read;
use manifestdocs_core::tree::{Tree, node_files_to_read, read_tree};

use crate::git;
use crate::locate::Roots;
use crate::report::relative;

/// Where the tree is read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocsSource {
    /// Inside the docs clone, or the tree isn't in git.
    WorkingTree,
    /// `origin/HEAD`, or the `--ref` given.
    Commit {
        reference: String,
        /// The short ID.
        commit: String,
        /// `YYYY-MM-DD`.
        date: String,
    },
}

/// The tree a command reads, and the version of it.
pub struct Docs {
    pub roots: Roots,
    pub source: DocsSource,
    files: Files,
}

enum Files {
    /// The tracked files are listed alongside the walk, since `.gitignore` can match them.
    WorkingTree { tracked: BTreeSet<RepoPath> },
    /// Each file in the commit with its blob's ID, and the one process that reads blobs.
    Commit {
        blobs: BTreeMap<RepoPath, String>,
        reader: RefCell<git::Blobs>,
    },
}

/// The version a command reads when it isn't given `--ref`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByDefault {
    /// The docs clone's working tree, wherever the command runs, so authors see their edits.
    WorkingTree,
    /// From another repo, the docs clone's default branch, `origin/HEAD`, so agents read
    /// only merged docs. Inside the docs clone, its working tree.
    DefaultBranch,
}

/// The tree at `roots`, read from the version `reference` names, or else as `by_default`
/// says. Never fetches.
pub fn open(
    roots: Roots,
    reference: Option<&str>,
    by_default: ByDefault,
    cwd: &Path,
) -> Result<Docs> {
    if let Some(reference) = reference.filter(|reference| reference.starts_with('-')) {
        bail!("`--ref` can't start with `-`: `{reference}`");
    }
    let clone = roots.repo_root.clone();
    let shown = relative(&clone, cwd);
    if !in_git(&clone, &shown)? {
        if reference.is_some() {
            let tree = relative(&roots.tree_root, cwd);
            bail!("`--ref` needs a docs clone, and the tree `{tree}` isn't in a git repo");
        }
        return Ok(working_tree(roots, BTreeSet::new()));
    }
    let default_branch = by_default == ByDefault::DefaultBranch && roots.from_another_repo;
    let reference = match reference {
        Some(reference) => reference.to_owned(),
        None if default_branch => default_branch_of(&clone, &shown)?,
        None => {
            let tracked = git::tracked(&clone).into_iter();
            let tracked = tracked.filter_map(|path| RepoPath::new(&path)).collect();
            return Ok(working_tree(roots, tracked));
        }
    };
    at_commit(roots, reference, &shown)
}

/// The branch `origin/HEAD` names in `clone`, such as `origin/main`, which `shown` names for
/// the hint when it isn't set.
pub fn default_branch_of(clone: &Path, shown: &str) -> Result<String> {
    git::origin_head(clone).ok_or_else(|| {
        anyhow!(
            "the docs clone's `origin/HEAD` isn't set, so its default branch is unknown; \
             set it with `git -C {shown} remote set-head origin --auto`"
        )
    })
}

/// Whether the clone is in git. A folder with no `.git` isn't, and doesn't need git to run.
fn in_git(clone: &Path, shown: &str) -> Result<bool> {
    if !clone.join(".git").exists() {
        return Ok(false);
    }
    git::is_repo_root(clone).with_context(|| format!("can't read the docs clone `{shown}`"))
}

fn at_commit(roots: Roots, reference: String, shown: &str) -> Result<Docs> {
    let clone = roots.repo_root.clone();
    let commit = git::resolve(&clone, &reference)
        .ok_or_else(|| anyhow!("no commit `{reference}` in the docs clone `{shown}`"))?;
    let files = git::files_at(&clone, &commit.id)?.into_iter();
    let blobs = files.filter_map(|(path, blob)| Some((RepoPath::new(&path)?, blob)));
    Ok(Docs {
        files: Files::Commit {
            blobs: blobs.collect(),
            reader: RefCell::new(git::Blobs::open(&clone)?),
        },
        source: DocsSource::Commit {
            reference,
            commit: commit.short,
            date: commit.date,
        },
        roots,
    })
}

fn working_tree(roots: Roots, tracked: BTreeSet<RepoPath>) -> Docs {
    Docs {
        roots,
        source: DocsSource::WorkingTree,
        files: Files::WorkingTree { tracked },
    }
}

impl Docs {
    /// The text of the file at `path`, which fails when it's missing or isn't UTF-8.
    fn read(&self, path: &RepoPath) -> Result<String> {
        match &self.files {
            Files::WorkingTree { .. } => Ok(fs::read_to_string(
                self.roots.repo_root.join(path.as_str()),
            )?),
            Files::Commit { blobs, reader } => {
                let blob = blobs.get(path).context("not in the commit")?;
                Ok(String::from_utf8(reader.borrow_mut().read(blob)?)?)
            }
        }
    }

    /// `` in `<reference>` @ <commit>`` when reading a commit, for error messages.
    fn at(&self) -> String {
        match &self.source {
            DocsSource::WorkingTree => String::new(),
            DocsSource::Commit {
                reference, commit, ..
            } => format!(" in `{reference}` @ {commit}"),
        }
    }
}

pub fn read_config(docs: &Docs, cwd: &Path) -> Result<TreeConfig> {
    let shown = relative(&docs.roots.tree_root.join(config::FILE_NAME), cwd);
    let path = tree_root(&docs.roots)?
        .join(config::FILE_NAME)
        .context("the tree's folder path isn't valid")?;
    let text = docs
        .read(&path)
        .with_context(|| format!("can't read {shown}{}", docs.at()))?;
    config::parse(&text).map_err(|error| match error.line {
        Some(line) => anyhow!("{shown}:{line}: {}", error.message),
        None => anyhow!("{shown}: {}", error.message),
    })
}

/// The tree, and the snapshot it was read from, which holds only the node files so far. This
/// is for the commands that read a tree without checking it. Supporting docs are listed but
/// not read, so one that isn't UTF-8 can't stop `tree` or `show`.
pub fn tree_and_snapshot(docs: &Docs, cwd: &Path) -> Result<(Tree, Snapshot)> {
    let config = read_config(docs, cwd)?;
    let mut snapshot = listed(docs)?;
    let node_files = node_files_to_read(&config, &snapshot.tree_root, &snapshot.paths);
    read_into(&mut snapshot, node_files, docs, cwd)?;
    Ok((read_tree(&config, &snapshot), snapshot))
}

/// Reads the docs a node links to into `snapshot`. A doc that can't be read as UTF-8 text is
/// skipped, so `context` lists it rather than failing.
pub fn read_linked_docs(snapshot: &mut Snapshot, paths: &[RepoPath], docs: &Docs) {
    for path in paths {
        if let Ok(text) = docs.read(path) {
            snapshot.markdown.insert(path.clone(), text);
        }
    }
}

/// Lists the repo's files and reads the tree's Markdown.
pub fn snapshot(docs: &Docs, config: &TreeConfig, cwd: &Path) -> Result<Snapshot> {
    let mut snapshot = listed(docs)?;
    let in_tree = markdown_to_read(config, &snapshot.tree_root, &snapshot.paths);
    read_into(&mut snapshot, in_tree, docs, cwd)?;
    Ok(snapshot)
}

/// A snapshot of the repo's file list, with no Markdown read yet.
fn listed(docs: &Docs) -> Result<Snapshot> {
    let paths = match &docs.files {
        Files::WorkingTree { tracked } => {
            let mut paths = list_files(&docs.roots.repo_root)?;
            let root = &docs.roots.repo_root;
            let on_disk = |path: &&RepoPath| is_file(&root.join(path.as_str()));
            paths.extend(tracked.iter().filter(on_disk).cloned());
            paths
        }
        Files::Commit { blobs, .. } => blobs.keys().cloned().collect(),
    };
    Ok(Snapshot {
        tree_root: tree_root(&docs.roots)?,
        paths,
        markdown: BTreeMap::new(),
    })
}

fn tree_root(roots: &Roots) -> Result<RepoPath> {
    repo_path(&roots.repo_root, &roots.tree_root).context("the tree's folder path isn't UTF-8")
}

/// Reads the Markdown outside the tree that links point into with an anchor, so `check`
/// can check those anchors too.
pub fn read_anchor_targets(
    snapshot: &mut Snapshot,
    docs: &Docs,
    config: &TreeConfig,
    cwd: &Path,
) -> Result<()> {
    let outside = outside_anchor_targets(config, snapshot);
    read_into(snapshot, outside, docs, cwd)
}

/// Reads the tree's decision records into `snapshot`, so `show` can find a record and the
/// chain it supersedes.
pub fn read_records(
    snapshot: &mut Snapshot,
    docs: &Docs,
    config: &TreeConfig,
    cwd: &Path,
) -> Result<()> {
    let records = records_to_read(config, &snapshot.tree_root, &snapshot.paths);
    read_into(snapshot, records, docs, cwd)
}

fn read_into(
    snapshot: &mut Snapshot,
    paths: impl IntoIterator<Item = RepoPath>,
    docs: &Docs,
    cwd: &Path,
) -> Result<()> {
    for path in paths {
        let text = docs.read(&path).with_context(|| {
            let file = relative(&docs.roots.repo_root.join(path.as_str()), cwd);
            match &docs.source {
                DocsSource::WorkingTree => format!("can't read {file} as UTF-8 text"),
                DocsSource::Commit { .. } => format!("can't read {file}{}", docs.at()),
            }
        })?;
        snapshot.markdown.insert(path, text);
    }
    Ok(())
}

// GitHub shows dotfiles and applies only the repo's own .gitignore files, so the walk does
// the same. A folder with its own .git is a nested repo or submodule, which GitHub shows as
// a pointer rather than files.
fn list_files(repo_root: &Path) -> Result<BTreeSet<RepoPath>> {
    let root = repo_root.to_path_buf();
    let walk = WalkBuilder::new(repo_root)
        .hidden(false)
        .ignore(false)
        .parents(false)
        .git_global(false)
        .git_exclude(false)
        .filter_entry(move |entry| !is_git_folder(entry) && !is_nested_repo(entry, &root))
        .build();
    let mut paths = BTreeSet::new();
    for entry in walk {
        let entry = entry?;
        let is_file = entry.file_type().is_some_and(|kind| kind.is_file());
        // A file name that isn't UTF-8 can't be a node or a link target, so it's skipped.
        paths.extend(
            is_file
                .then(|| repo_path(repo_root, entry.path()))
                .flatten(),
        );
    }
    Ok(paths)
}

/// A regular file, not a symbolic link, as the walk counts files.
fn is_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file())
}

fn is_git_folder(entry: &DirEntry) -> bool {
    entry.file_name() == ".git"
}

fn is_nested_repo(entry: &DirEntry, repo_root: &Path) -> bool {
    let is_dir = entry.file_type().is_some_and(|kind| kind.is_dir());
    is_dir && entry.path() != repo_root && entry.path().join(".git").exists()
}

fn repo_path(repo_root: &Path, path: &Path) -> Option<RepoPath> {
    RepoPath::new(path.strip_prefix(repo_root).ok()?.to_str()?)
}
