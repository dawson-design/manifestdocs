//! The docs clone's git state, read with `git`: its commits and their files, and the branch
//! and commit for the context header. Nothing here fetches or writes.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use anyhow::{Context, Result, anyhow, bail};
use manifestdocs_core::context::DocsVersion;

/// The version of the working tree at `tree_root`. `uncommitted` covers the tree's folder and
/// `docs`, the files outside it that the context loads. Without git, or without a commit,
/// it's `NotInGit`, so a tree still gets its context.
pub fn version(tree_root: &Path, docs: &[PathBuf]) -> DocsVersion {
    let log = ["log", "-1", "--no-show-signature", "--format=%h %cs"];
    let Some(head) = git(tree_root, &log) else {
        return DocsVersion::NotInGit;
    };
    let Some((commit, date)) = head.split_once(' ') else {
        return DocsVersion::NotInGit;
    };
    let branch = git(tree_root, &["symbolic-ref", "--quiet", "--short", "HEAD"]);
    let mut status = vec![
        "status".into(),
        "--porcelain".into(),
        "--".into(),
        ".".into(),
    ];
    status.extend(docs.iter().map(|doc| doc.display().to_string()));
    let status: Vec<&str> = status.iter().map(String::as_str).collect();
    let changes = git(tree_root, &status);
    DocsVersion::WorkingTree {
        branch,
        commit: commit.to_owned(),
        date: date.to_owned(),
        uncommitted: changes.is_some_and(|changes| !changes.is_empty()),
        default_branch: origin_head(tree_root)
            .and_then(|head| head.strip_prefix("origin/").map(str::to_owned)),
    }
}

/// Whether `folder`, which holds a `.git`, is the top folder of a git repo. A `.git` that
/// isn't a repo counts as outside git, but a git that can't run, or that refuses the repo,
/// such as for `safe.directory`, is an error.
pub fn is_repo_root(folder: &Path) -> Result<bool> {
    let output = command(folder)
        .env("LC_ALL", "C")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("can't run `git`")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not a git repository") {
            return Ok(false);
        }
        bail!("`git` can't read it: {}", stderr.trim());
    }
    let top = String::from_utf8(output.stdout).ok();
    let top = top.and_then(|top| fs::canonicalize(top.trim()).ok());
    Ok(top.is_some_and(|top| fs::canonicalize(folder).is_ok_and(|folder| folder == top)))
}

/// The branch `origin/HEAD` names, such as `origin/main`, when it's set.
pub fn origin_head(clone: &Path) -> Option<String> {
    git(
        clone,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    )
}

/// A commit `reference` names in the clone.
pub struct Commit {
    /// The full ID, for reading the commit's files.
    pub id: String,
    /// The short ID, for people.
    pub short: String,
    /// The commit's date, as `YYYY-MM-DD`.
    pub date: String,
}

/// The commit `reference` names in `clone`, when there's one.
pub fn resolve(clone: &Path, reference: &str) -> Option<Commit> {
    // No ref name starts with `-`, so this one can't be read as an option. The caller
    // rejects one first with a clearer message; this guards every other caller.
    if reference.starts_with('-') {
        return None;
    }
    let commit = format!("{reference}^{{commit}}");
    let id = git(clone, &["rev-parse", "--verify", "--quiet", &commit])?;
    let log = ["log", "-1", "--no-show-signature", "--format=%h %cs", &id];
    let head = git(clone, &log)?;
    let (short, date) = head.split_once(' ')?;
    Some(Commit {
        short: short.to_owned(),
        date: date.to_owned(),
        id,
    })
}

/// Every file in commit `id`, from the repo root, with its blob's ID. Submodules and
/// symbolic links aren't files, as in the working tree.
pub fn files_at(clone: &Path, id: &str) -> Result<Vec<(String, String)>> {
    let listing = output(clone, &["ls-tree", "-r", "-z", "--full-tree", id])
        .with_context(|| format!("can't list the files in commit {id}"))?;
    let mut files = Vec::new();
    for entry in listing.split(|byte| *byte == 0).filter(|e| !e.is_empty()) {
        // A path that isn't UTF-8 can't be a node or a link target, so it's skipped.
        let Ok(entry) = std::str::from_utf8(entry) else {
            continue;
        };
        let (meta, path) = entry
            .split_once('\t')
            .ok_or_else(|| anyhow!("unexpected `git ls-tree` output: {entry}"))?;
        if let [mode, "blob", blob] = meta.split(' ').collect::<Vec<_>>()[..]
            && mode != "120000"
        {
            files.push((path.to_owned(), blob.to_owned()));
        }
    }
    Ok(files)
}

/// The files in git's index under `repo_root`, from the repo root, so a tracked file that
/// `.gitignore` matches is still listed. Empty outside git.
pub fn tracked(repo_root: &Path) -> Vec<String> {
    let Some(listing) = output(repo_root, &["ls-files", "-z"]) else {
        return Vec::new();
    };
    let paths = listing.split(|byte| *byte == 0).filter(|p| !p.is_empty());
    let paths = paths.filter_map(|path| std::str::from_utf8(path).ok());
    paths.map(str::to_owned).collect()
}

/// One `git cat-file --batch` process, which reads blobs one at a time.
pub struct Blobs {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
}

impl Blobs {
    pub fn open(clone: &Path) -> Result<Self> {
        let mut child = command(clone)
            .args(["cat-file", "--batch"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("can't run `git cat-file`")?;
        let input = child.stdin.take();
        let output = child.stdout.take().context("can't read `git cat-file`")?;
        Ok(Self {
            child,
            input,
            output: BufReader::new(output),
        })
    }

    /// The contents of blob `id`. After an error the process's output can be out of step,
    /// so it reads nothing more.
    pub fn read(&mut self, id: &str) -> Result<Vec<u8>> {
        let contents = self.exchange(id);
        if contents.is_err() {
            self.input.take();
        }
        contents
    }

    fn exchange(&mut self, id: &str) -> Result<Vec<u8>> {
        let input = self.input.as_mut().context("`git cat-file` has stopped")?;
        writeln!(input, "{id}")
            .and_then(|()| input.flush())
            .context("can't write to `git cat-file`")?;
        let mut header = String::new();
        self.output.read_line(&mut header)?;
        let size = match header.trim_end().split(' ').collect::<Vec<_>>()[..] {
            [_, "blob", size] => size.parse::<usize>()?,
            [_, "missing"] => {
                bail!("the clone doesn't have blob {id}, and manifestdocs never fetches")
            }
            _ => bail!("can't read blob {id}: {}", header.trim_end()),
        };
        // The contents, then a newline.
        let mut contents = vec![0; size + 1];
        self.output.read_exact(&mut contents)?;
        contents.truncate(size);
        Ok(contents)
    }
}

impl Drop for Blobs {
    fn drop(&mut self) {
        // After an error the process can be blocked writing output nobody reads, so it's
        // killed rather than waited on.
        self.input.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Variables that point git at a repo other than the folder it's given, such as when a
/// hook runs the tool with `GIT_DIR` set to the code repo.
const REPO_VARIABLES: [&str; 5] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_COMMON_DIR",
];

/// `git -C folder`, reading only that folder's repo. `GIT_NO_LAZY_FETCH` stops a partial
/// clone fetching a missing object, so the tool never reaches the network.
fn command(folder: &Path) -> Command {
    let mut command = Command::new("git");
    command.arg("-C").arg(folder).env("GIT_NO_LAZY_FETCH", "1");
    for name in REPO_VARIABLES {
        command.env_remove(name);
    }
    command
}

/// `git`'s trimmed output, or `None` when it fails or isn't installed.
fn git(folder: &Path, args: &[&str]) -> Option<String> {
    let stdout = String::from_utf8(output(folder, args)?).ok()?;
    Some(stdout.trim().to_owned())
}

/// `git`'s output, or `None` when it fails or isn't installed.
fn output(folder: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = command(folder).args(args).output().ok()?;
    output.status.success().then_some(output.stdout)
}
