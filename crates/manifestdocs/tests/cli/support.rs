use std::ffi::OsString;
use std::fs;
use std::iter;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// A throwaway repo in the temp folder, removed when dropped.
pub struct Fixture {
    root: PathBuf,
}

pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Fixture {
    /// Writes `files` under a new folder with an empty `.git` folder, so the folder reads as a repo root.
    pub fn new(name: &str, files: &[(&str, &str)]) -> Self {
        let root = std::env::temp_dir().join(format!("mfd-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".git")).expect("create the fixture");
        for (path, text) in files {
            let path = root.join(path);
            fs::create_dir_all(path.parent().expect("a file has a folder"))
                .expect("create a folder");
            fs::write(&path, text).expect("write a fixture file");
        }
        Self { root }
    }

    /// The absolute path of `relative` inside the fixture.
    pub fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Runs `manifestdocs check` from `folder`, a path relative to the fixture root.
    pub fn check(&self, folder: &str) -> Run {
        self.run(folder, &["check"])
    }

    /// Runs `manifestdocs` with `args` from `folder`, with the fixture root as the home folder.
    pub fn run(&self, folder: &str, args: &[&str]) -> Run {
        run_in(&self.root.join(folder), &self.root, args)
    }

    /// Runs `manifestdocs` as `run` does, with the fixture's fake `gh` first on `PATH`.
    pub fn run_with_gh(&self, folder: &str, args: &[&str]) -> Run {
        run_in_with_gh(&self.root.join(folder), &self.root, args, &self.fake_gh())
    }

    /// Runs `manifestdocs` as `run` does, with a folder holding only `git` as the whole
    /// `PATH`, so no `gh` can be found but the docs clone can still be read.
    pub fn run_without_gh(&self, folder: &str, args: &[&str]) -> Run {
        let only_git = self.fake_gh().join("only-git");
        fs::create_dir_all(&only_git).expect("create a folder for git");
        let link = only_git.join("git");
        if !link.exists() {
            std::os::unix::fs::symlink(real_git(), &link).expect("link to git");
        }
        run_with_path(&self.root.join(folder), &self.root, args, only_git.into())
    }

    /// Has the fake `gh` know `issue` (`owner/name#number`) on `host` as an open issue.
    pub fn gh_issue(&self, host: &str, issue: &str) {
        let (repo, number) = issue.split_once('#').expect("an issue reference");
        let json = format!(
            r#"{{"number":{number},"title":"Issue {number}","state":"OPEN","createdAt":"2026-10-01T09:30:00Z","body":"","issueType":null,"parent":null,"subIssues":{{"nodes":[]}}}}"#
        );
        self.write_gh(&format!("issues/{host}/{repo}/{number}.json"), &json);
    }

    /// Has the fake `gh` answer that `issue` on `host` isn't an issue, as GitHub does for a
    /// number with no issue or a pull request.
    pub fn gh_no_issue(&self, host: &str, issue: &str) {
        let (repo, number) = issue.split_once('#').expect("an issue reference");
        self.write_gh(&format!("issues/{host}/{repo}/{number}.json"), "null");
    }

    /// Has the fake `gh` answer that `repo` on `host` doesn't exist.
    pub fn gh_no_repo(&self, host: &str, repo: &str) {
        self.write_gh(&format!("issues/{host}/{repo}.missing"), "");
    }

    /// Has the fake `gh` answer a search of `repo` on `host` with one page of `issues`, as
    /// numbers and bodies.
    pub fn gh_search(&self, host: &str, repo: &str, issues: &[(u64, &str)]) {
        self.gh_search_page(host, repo, None, issues, None, issues.len());
    }

    /// Has the fake `gh` answer a search of `repo` on `host` from `cursor` (`None` for the
    /// first page) with `issues`, `next` as the cursor of the page after, if any, and `total`
    /// as GitHub's count of every result.
    pub fn gh_search_page(
        &self,
        host: &str,
        repo: &str,
        cursor: Option<&str>,
        issues: &[(u64, &str)],
        next: Option<&str>,
        total: usize,
    ) {
        let nodes: Vec<String> = issues
            .iter()
            .map(|(number, body)| format!(r#"{{"number":{number},"body":{}}}"#, json(body)))
            .collect();
        let page_info = match next {
            Some(next) => format!(r#"{{"hasNextPage":true,"endCursor":{}}}"#, json(next)),
            None => r#"{"hasNextPage":false,"endCursor":null}"#.to_owned(),
        };
        let response = format!(
            r#"{{"data":{{"search":{{"issueCount":{total},"pageInfo":{page_info},"nodes":[{}]}}}}}}"#,
            nodes.join(",")
        );
        self.gh_search_answer(host, repo, cursor, &response);
    }

    /// Has the fake `gh` answer a search of `repo` on `host` from `cursor` (`None` for the
    /// first page) with `response`, the JSON `gh api graphql` prints. An answer with
    /// `errors` exits 1, as `gh` does.
    pub fn gh_search_answer(&self, host: &str, repo: &str, cursor: Option<&str>, response: &str) {
        let page = cursor.unwrap_or("first");
        self.write_gh(&format!("search/{host}/{repo}/{page}.json"), response);
    }

    /// Has the fake `gh` answer that it can't read `issue` on `host`, as GitHub does with a
    /// FORBIDDEN error for one issue in a batch.
    pub fn gh_unreadable_issue(&self, host: &str, issue: &str) {
        let (repo, number) = issue.split_once('#').expect("an issue reference");
        self.write_gh(&format!("issues/{host}/{repo}/{number}.json"), "forbidden");
    }

    /// Has the fake `gh` fail every request as `gh` does when no account is logged in.
    pub fn gh_logged_out(&self) {
        self.write_gh("logged-out", "");
    }

    /// Has the fake `gh` fail every request as `gh` does when GitHub refuses its token.
    pub fn gh_unauthorized(&self) {
        self.write_gh("unauthorized", "");
    }

    /// Writes the fixture's fake `gh` and returns its folder, for a test that builds its own
    /// `PATH`.
    pub fn gh_folder(&self) -> PathBuf {
        write_fake_gh(&self.fake_gh())
    }

    /// The arguments of each call the fake `gh` received, one line per call.
    pub fn gh_calls(&self) -> Vec<String> {
        let log = fs::read_to_string(self.fake_gh().join("calls.log")).unwrap_or_default();
        log.lines().map(str::to_owned).collect()
    }

    fn write_gh(&self, relative: &str, text: &str) {
        let file = self.fake_gh().join(relative);
        fs::create_dir_all(file.parent().expect("a file has a folder")).expect("create a folder");
        fs::write(file, text).expect("write a fake gh answer");
    }

    // Under `.git`, which the tree walk skips, so the fake isn't part of the tree.
    fn fake_gh(&self) -> PathBuf {
        self.root.join(".git/fake-gh")
    }
}

/// Runs `manifestdocs` with `args` in `folder`, with `home` as the home folder. Git stops
/// looking for a repo at `home`'s parent, so a temp folder inside some other repo can't
/// change what a fixture reports. A `gh` that always fails is first on `PATH`, so no test
/// reaches GitHub.
pub fn run_in(folder: &Path, home: &Path, args: &[&str]) -> Run {
    output(&mut command(folder, home, args))
}

/// Runs `manifestdocs` as `run_in` does, with a fake `gh` in `fake_gh` first on `PATH`. The
/// fake answers read-only `gh api graphql` requests from the files the `Fixture::gh_*`
/// helpers write, and fails any request it has no file for, so a test can't reach GitHub.
pub fn run_in_with_gh(folder: &Path, home: &Path, args: &[&str], fake_gh: &Path) -> Run {
    run_with_path(folder, home, args, path_with(&write_fake_gh(fake_gh)))
}

/// Writes the fake `gh` into `fake_gh/bin` and returns that folder.
fn write_fake_gh(fake_gh: &Path) -> PathBuf {
    let bin = fake_gh.join("bin");
    write_script(&bin.join("gh"), &fake_gh_script(fake_gh));
    bin
}

fn run_with_path(folder: &Path, home: &Path, args: &[&str], path: OsString) -> Run {
    output(command(folder, home, args).env("PATH", path))
}

fn command(folder: &Path, home: &Path, args: &[&str]) -> Command {
    let ceiling = home.parent().unwrap_or(home);
    let mut command = Command::new(env!("CARGO_BIN_EXE_manifestdocs"));
    command
        .args(args)
        .current_dir(folder)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("GIT_CEILING_DIRECTORIES", ceiling)
        .env("PATH", path_without_gh());
    command
}

/// The test's `PATH` with a `gh` that always fails first, as `run_in` sets it.
pub fn path_without_gh() -> OsString {
    path_with(blocked_gh())
}

/// The `git` on the test's `PATH`.
fn real_git() -> PathBuf {
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .map(|folder| folder.join("git"))
        .find(|git| git.is_file())
        .expect("git is on PATH")
}

/// The test's `PATH` with `folder` first.
fn path_with(folder: &Path) -> OsString {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let folders = iter::once(folder.to_path_buf()).chain(std::env::split_paths(&path));
    std::env::join_paths(folders).expect("a valid PATH")
}

/// A folder holding a `gh` that always fails, for the front of any `PATH` a test sets, so
/// no test reaches GitHub. Written once per test process, before any test runs it.
pub fn blocked_gh() -> &'static Path {
    static FOLDER: OnceLock<PathBuf> = OnceLock::new();
    FOLDER.get_or_init(|| {
        let folder = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("blocked-gh-{}", std::process::id()));
        let script =
            "#!/bin/sh\necho 'gh is blocked in tests; use Fixture::run_with_gh' >&2\nexit 1\n";
        write_script(&folder.join("gh"), script);
        folder
    })
}

fn write_script(file: &Path, text: &str) {
    fs::create_dir_all(file.parent().expect("a file has a folder")).expect("create a folder");
    fs::write(file, text).expect("write a script");
    fs::set_permissions(file, fs::Permissions::from_mode(0o755)).expect("make a script runnable");
}

fn output(command: &mut Command) -> Run {
    let output = command.output().expect("the binary runs");
    Run {
        code: output.status.code().expect("the binary exits with a code"),
        stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr is UTF-8"),
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// `text` as a JSON string.
fn json(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    format!("\"{escaped}\"")
}

// Logs each call, then answers a read-only `api graphql` request from the fixture's files:
// a search of one repo, or a batch of issues in one repo, each asked about as `i<number>`.
// Anything else, including a mutation or an issue with no file, fails.
fn fake_gh_script(fake_gh: &Path) -> String {
    let dir = fake_gh.display();
    format!(
        r#"#!/bin/sh
PATH=/usr/bin:/bin
dir='{dir}'
printf '%s\n' "$*" >> "$dir/calls.log"
fail() {{ echo "fake gh: $*" >&2; exit 1; }}
if [ -e "$dir/logged-out" ]; then
  echo 'To get started with GitHub CLI, please run:  gh auth login' >&2
  exit 4
fi
if [ -e "$dir/unauthorized" ]; then
  echo 'gh: HTTP 401: Bad credentials (https://api.github.com/graphql)' >&2
  exit 1
fi
[ "$1 $2" = "api graphql" ] || fail "unexpected command: $*"
shift 2
host= query= owner= name= q= cursor=
while [ $# -gt 1 ]; do
  case "$1=$2" in
    --hostname=*) host=$2 ;;
    -f=query=*) query=${{2#query=}} ;;
    -f=owner=*) owner=${{2#owner=}} ;;
    -f=name=*) name=${{2#name=}} ;;
    -f=q=*) q=${{2#q=}} ;;
    -f=cursor=*) cursor=${{2#cursor=}} ;;
    *) fail "unexpected argument: $1 $2" ;;
  esac
  shift 2
done
[ $# -eq 0 ] || fail "unexpected argument: $1"
case "$query" in
  *mutation*) fail "the binary never writes to GitHub" ;;
  query*) ;;
  *) fail "not a query: $query" ;;
esac
if [ -n "$q" ]; then
  repo=$(printf '%s\n' "$q" | sed -n 's/.*repo:\([^ ]*\).*/\1/p')
  file="$dir/search/$host/$repo/${{cursor:-first}}.json"
  [ -f "$file" ] || fail "no search results for $repo on $host"
  cat "$file"
  if grep -q '"errors"' "$file"; then exit 1; fi
  exit 0
fi
repo="$dir/issues/$host/$owner/$name"
if [ -e "$repo.missing" ]; then
  printf '%s' '{{"data":{{"repository":null}},"errors":[{{"type":"NOT_FOUND","path":["repository"],"message":"Could not resolve to a Repository."}}]}}'
  exit 1
fi
data= errors=
for number in $(printf '%s\n' "$query" | grep -o 'issue(number: [0-9]*)' | tr -cd '0-9\n'); do
  file="$repo/$number.json"
  [ -f "$file" ] || fail "no answer for $owner/$name#$number on $host"
  issue=$(cat "$file")
  if [ "$issue" = forbidden ]; then
    issue=null
    errors="$errors${{errors:+,}}{{\"type\":\"FORBIDDEN\",\"path\":[\"repository\",\"i$number\"],\"message\":\"Resource not accessible by integration\"}}"
  elif [ "$issue" = null ]; then
    errors="$errors${{errors:+,}}{{\"type\":\"NOT_FOUND\",\"path\":[\"repository\",\"i$number\"],\"message\":\"Could not resolve to an Issue with the number of $number.\"}}"
  fi
  data="$data${{data:+,}}\"i$number\":$issue"
done
[ -n "$data" ] || fail "no issues asked about: $query"
if [ -z "$errors" ]; then
  printf '{{"data":{{"repository":{{%s}}}}}}' "$data"
  exit 0
fi
# `gh api` exits 1 when the answer has GraphQL errors.
printf '{{"data":{{"repository":{{%s}}}},"errors":[%s]}}' "$data" "$errors"
exit 1
"#
    )
}

/// A node file with valid frontmatter.
pub fn node(id: &str, title: &str) -> String {
    format!(
        "---\nid: {id}\ntitle: \"{title}\"\nstatus: draft\nsummary: \"{title} summary.\"\n---\n\n# {title}\n"
    )
}

impl Fixture {
    /// Runs `manifestdocs` with `args` from `folder`, with `stdin` as its input and `env` set,
    /// such as `MANIFESTDOCS_TEST_DATE` to pin the date `decide` writes.
    pub fn run_with(&self, folder: &str, args: &[&str], stdin: &str, env: &[(&str, &str)]) -> Run {
        use std::io::Write as _;
        use std::process::Stdio;

        let mut child = command(&self.root.join(folder), &self.root, args)
            .envs(env.iter().copied())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the binary runs");
        let mut input = child.stdin.take().expect("stdin is piped");
        // The binary may exit without reading its input, which closes the pipe.
        let _ = input.write_all(stdin.as_bytes());
        drop(input);
        let output = child.wait_with_output().expect("the binary exits");
        Run {
            code: output.status.code().expect("the binary exits with a code"),
            stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
            stderr: String::from_utf8(output.stderr).expect("stderr is UTF-8"),
        }
    }

    /// The text of `relative` inside the fixture.
    pub fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.path(relative)).expect("read a fixture file")
    }
}
