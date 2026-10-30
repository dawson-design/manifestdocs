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
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// Variables that change how git reads a pathspec, such as `GIT_LITERAL_PATHSPECS`, which
/// would turn off the magic `PathSet` relies on.
const PATHSPEC_VARIABLES: [&str; 4] = [
    "GIT_LITERAL_PATHSPECS",
    "GIT_GLOB_PATHSPECS",
    "GIT_NOGLOB_PATHSPECS",
    "GIT_ICASE_PATHSPECS",
];

/// `git -C folder`, reading only that folder's repo. `GIT_NO_LAZY_FETCH` stops a partial
/// clone fetching a missing object, so the tool never reaches the network.
fn command(folder: &Path) -> Command {
    let mut command = Command::new("git");
    command.arg("-C").arg(folder).env("GIT_NO_LAZY_FETCH", "1");
    for name in REPO_VARIABLES.iter().chain(&PATHSPEC_VARIABLES) {
        command.env_remove(name);
    }
    command
}

/// The branch, default branch, commits ahead of it and uncommitted changes of the clone
/// holding `folder`, for `decide`'s warnings. `None` without git or a commit.
pub fn clone_state(folder: &Path) -> Option<manifestdocs_core::decide::CloneState> {
    git(folder, &["rev-parse", "--verify", "--quiet", "HEAD"])?;
    let changes = git(folder, &["status", "--porcelain"])?;
    let ahead = git(
        folder,
        &["rev-list", "--count", "refs/remotes/origin/HEAD..HEAD"],
    );
    Some(manifestdocs_core::decide::CloneState {
        branch: git(folder, &["symbolic-ref", "--quiet", "--short", "HEAD"]),
        default_branch: origin_head(folder)
            .and_then(|head| head.strip_prefix("origin/").map(str::to_owned)),
        ahead_of_default: ahead.is_some_and(|count| count != "0"),
        uncommitted: !changes.is_empty(),
    })
}

/// The files under a folder of the repo, less the files and folders in `excluded`, all from
/// the repo root. Names are literal, so `*` in one matches only itself.
pub struct PathSet {
    /// `""` for the whole repo.
    pub folder: String,
    pub excluded: Vec<String>,
}

impl PathSet {
    fn pathspecs(&self) -> Vec<String> {
        let folder = match self.folder.as_str() {
            "" => ":(top)".to_owned(),
            folder => format!(":(top,literal){folder}"),
        };
        let excluded = self.excluded.iter();
        let excluded = excluded.map(|path| format!(":(top,literal,exclude){path}"));
        std::iter::once(folder).chain(excluded).collect()
    }
}

/// The latest commit in `head`'s history that changed a file in `paths`, leaving out the
/// commits in `base`'s history. Without `base`, every commit in `head`'s history counts.
/// `head` and `base` are commit IDs, as `resolve` gives them.
pub fn latest_change(
    clone: &Path,
    base: Option<&str>,
    head: &str,
    paths: &PathSet,
) -> Result<Option<Commit>> {
    let not_base = base.map(|base| format!("^{base}"));
    let mut args = vec!["rev-list", "-1", head];
    args.extend(not_base.as_deref());
    args.push("--");
    let pathspecs = paths.pathspecs();
    args.extend(pathspecs.iter().map(String::as_str));
    found(clone, &run(clone, &args)?)
}

/// The last commit on `head`'s first-parent line, its default branch's own history, made
/// before `time`, such as an issue's creation time in RFC 3339. `None` when every commit is
/// later. Merged branches' commits are skipped, since they weren't on the branch at `time`.
pub fn commit_before(clone: &Path, head: &str, time: &str) -> Result<Option<Commit>> {
    let before = format!("--before={time}");
    let args = ["rev-list", "-1", "--first-parent", &before, head];
    found(clone, &run(clone, &args)?)
}

/// Whether commit `ancestor` is in `head`'s history, `head` included. Both are commit IDs.
pub fn is_ancestor(clone: &Path, ancestor: &str, head: &str) -> Result<bool> {
    let args = ["merge-base", "--is-ancestor", ancestor, head];
    let output = command(clone).args(args).output();
    let output = output.context("can't run `git`")?;
    // `--is-ancestor` exits 1 for "no" and higher for an error.
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("`git merge-base` failed: {}", stderr.trim())
        }
    }
}

/// The text of the file at `path`, from the repo root, in commit `id`. Bytes that aren't
/// UTF-8 are replaced, since the text is only compared.
pub fn text_at(clone: &Path, id: &str, path: &str) -> Result<String> {
    let object = format!("{id}:{path}");
    let contents = run_bytes(clone, &["cat-file", "blob", &object])?;
    Ok(String::from_utf8_lossy(&contents).into_owned())
}

/// Whether the clone is shallow, so commits before its cut-off aren't in it.
pub fn is_shallow(clone: &Path) -> Result<bool> {
    Ok(run(clone, &["rev-parse", "--is-shallow-repository"])? == "true")
}

/// The commit `id` names, or `None` for no ID.
fn found(clone: &Path, id: &str) -> Result<Option<Commit>> {
    if id.is_empty() {
        return Ok(None);
    }
    let commit = resolve(clone, id).ok_or_else(|| anyhow!("can't read commit {id}"))?;
    Ok(Some(commit))
}

/// A file a diff changed, by its path from the repo root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    pub lines: Lines,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lines {
    Text { added: u64, removed: u64 },
    Binary,
}

/// The files in `paths` that differ between `base` and `head`, in path order. Without `base`,
/// every file in `head` is added. A renamed file is one deleted and one added.
pub fn diff_summary(
    clone: &Path,
    base: Option<&str>,
    head: &str,
    paths: &PathSet,
) -> Result<Vec<FileChange>> {
    let empty;
    let base = match base {
        Some(base) => base,
        None => {
            empty = run(clone, &["hash-object", "-t", "tree", "--stdin"])?;
            &empty
        }
    };
    // `diff-tree` is plumbing, so diff settings in the user's config don't change its output.
    let mut args = ["diff-tree", "-r", "--numstat", "--no-renames", "-z"].to_vec();
    args.extend([base, head, "--"]);
    let pathspecs = paths.pathspecs();
    args.extend(pathspecs.iter().map(String::as_str));
    file_changes(&run_bytes(clone, &args)?)
}

/// `diff-tree --numstat -z` output, one entry per file.
fn file_changes(listing: &[u8]) -> Result<Vec<FileChange>> {
    let entries = listing.split(|byte| *byte == 0).filter(|e| !e.is_empty());
    // A path that isn't UTF-8 can't be a node's file, so it's skipped.
    let entries = entries.filter_map(|entry| std::str::from_utf8(entry).ok());
    entries.map(file_change).collect()
}

/// One `--numstat` line: lines added, lines removed and the path, or `-` for both counts of
/// a binary file.
fn file_change(entry: &str) -> Result<FileChange> {
    let unexpected = || anyhow!("unexpected `git diff-tree` output: {entry}");
    let mut fields = entry.splitn(3, '\t');
    let (Some(added), Some(removed), Some(path)) = (fields.next(), fields.next(), fields.next())
    else {
        return Err(unexpected());
    };
    let lines = match (added, removed) {
        ("-", "-") => Lines::Binary,
        _ => Lines::Text {
            added: added.parse().map_err(|_| unexpected())?,
            removed: removed.parse().map_err(|_| unexpected())?,
        },
    };
    Ok(FileChange {
        path: path.to_owned(),
        lines,
    })
}

/// `git`'s trimmed output, or an error with git's own message, so a missing object in a
/// partial clone isn't read as no answer.
fn run(folder: &Path, args: &[&str]) -> Result<String> {
    let stdout = String::from_utf8(run_bytes(folder, args)?);
    Ok(stdout
        .context("`git` printed text that isn't UTF-8")?
        .trim()
        .to_owned())
}

fn run_bytes(folder: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = command(folder).args(args).output();
    let output = output.context("can't run `git`")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`git {}` failed: {}", args[0], stderr.trim());
    }
    Ok(output.stdout)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A repo in the temp folder, removed when dropped.
    struct Repo(PathBuf);

    impl Repo {
        fn new(name: &str) -> Self {
            let name = format!("mfd-git-{name}-{}", std::process::id());
            let root = std::env::temp_dir().join(name);
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            let repo = Self(root);
            repo.git(&["init", "-q", "-b", "main"], "2026-01-01T00:00:00Z");
            repo
        }

        fn write(&self, path: &str, contents: &[u8]) {
            let path = self.0.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }

        /// Commits everything at `date`, returning the commit's ID.
        fn commit(&self, date: &str) -> String {
            self.git(&["add", "-A"], date);
            self.git(&["commit", "-q", "--allow-empty", "-m", date], date);
            self.git(&["rev-parse", "HEAD"], date)
        }

        fn git(&self, args: &[&str], date: &str) -> String {
            let output = command(&self.0)
                .args(["-c", "user.name=Sam", "-c", "user.email=sam@example.com"])
                .args(["-c", "commit.gpgsign=false"])
                .args(args)
                .env("HOME", &self.0)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_AUTHOR_DATE", date)
                .env("GIT_COMMITTER_DATE", date)
                .output()
                .unwrap();
            assert!(output.status.success(), "git {args:?}: {output:?}");
            String::from_utf8(output.stdout).unwrap().trim().to_owned()
        }
    }

    impl Drop for Repo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn paths(folder: &str, excluded: &[&str]) -> PathSet {
        PathSet {
            folder: folder.to_owned(),
            excluded: excluded.iter().map(|path| (*path).to_owned()).collect(),
        }
    }

    fn id(commit: Option<Commit>) -> Option<String> {
        commit.map(|commit| commit.id)
    }

    #[test]
    fn finds_the_latest_change_to_a_folder_less_its_exclusions() {
        let repo = Repo::new("latest");
        for path in ["a/a.md", "a/b/b.md", "a/decisions/d.md", "root.md"] {
            repo.write(path, b"1\n");
        }
        let first = repo.commit("2026-10-01T10:00:00Z");
        repo.write("a/b/b.md", b"2\n");
        repo.commit("2026-10-03T10:00:00Z");
        repo.write("a/decisions/d.md", b"2\n");
        repo.commit("2026-10-05T10:00:00Z");
        repo.write("root.md", b"2\n");
        let last = repo.commit("2026-10-07T10:00:00Z");
        let a = paths("a", &["a/b", "a/decisions"]);
        let all_but_a = paths("", &["a"]);

        let latest = |base, paths| latest_change(&repo.0, base, &last, paths).unwrap();

        assert_eq!(id(latest(None, &a)), Some(first.clone()));
        assert_eq!(id(latest(Some(&first), &a)), None);
        assert_eq!(id(latest(Some(&first), &all_but_a)), Some(last.clone()));
    }

    #[test]
    fn finds_the_last_commit_on_the_first_parent_line_before_a_time() {
        let repo = Repo::new("before");
        let first = repo.commit("2026-10-01T10:00:00Z");
        repo.git(&["checkout", "-q", "-b", "side"], "2026-10-02T10:00:00Z");
        repo.commit("2026-10-02T10:00:00Z");
        repo.git(&["checkout", "-q", "main"], "2026-10-03T10:00:00Z");
        let second = repo.commit("2026-10-03T10:00:00Z");
        let merge = ["merge", "-q", "--no-ff", "-m", "Merge", "side"];
        repo.git(&merge, "2026-10-05T10:00:00Z");
        let head = repo.git(&["rev-parse", "HEAD"], "2026-10-05T10:00:00Z");

        let before = |time| id(commit_before(&repo.0, &head, time).unwrap());

        assert_eq!(before("2026-10-02T12:00:00Z"), Some(first));
        assert_eq!(before("2026-10-04T00:00:00Z"), Some(second));
        assert_eq!(before("2026-09-01T00:00:00Z"), None);
    }

    #[test]
    fn summarises_the_files_changed_under_a_folder() {
        let repo = Repo::new("diff");
        repo.write("a/a.md", b"1\n");
        repo.write("a/flow.png", &[0, 1, 2]);
        repo.write("b.md", b"1\n");
        let first = repo.commit("2026-10-01T10:00:00Z");
        repo.write("a/a.md", b"2\n3\n");
        repo.write("a/new.md", b"1\n");
        repo.write("b.md", b"2\n");
        let second = repo.commit("2026-10-03T10:00:00Z");
        let text = |path: &str, added, removed| FileChange {
            path: path.to_owned(),
            lines: Lines::Text { added, removed },
        };

        let diff = |base| diff_summary(&repo.0, base, &second, &paths("a", &[])).unwrap();

        assert_eq!(
            diff(Some(&first)),
            [text("a/a.md", 2, 1), text("a/new.md", 1, 0)]
        );
        let binary = FileChange {
            path: "a/flow.png".to_owned(),
            lines: Lines::Binary,
        };
        assert_eq!(
            diff(None),
            [text("a/a.md", 2, 0), binary, text("a/new.md", 1, 0)]
        );
    }
}
