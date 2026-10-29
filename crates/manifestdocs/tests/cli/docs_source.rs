//! Which version of the docs the read commands read: the working tree, `origin/HEAD` or `--ref`.

use std::fs;
use std::process::Command;

use crate::support::{Fixture, Run, node};

/// From the code repo, the commands reach the docs clone through this pointer.
const POINTER: (&str, &str) = ("work/.manifestdocs/config.yaml", "tree: docs\n");

/// A workspace with a code repo, `work/web`, and a docs clone, `work/docs`, whose `main` is
/// pushed to a bare origin and is `origin/HEAD`. The Search node's title tells the versions
/// apart: "Search" on `main`, "Search, feature" on the checked-out `feature/x` branch, and
/// "Search, edited" uncommitted. Returns `main`'s and `feature/x`'s short commits.
pub fn workspace(name: &str) -> (Fixture, String, String) {
    let fixture = Fixture::new(
        name,
        &[
            POINTER,
            ("work/web/src/main.rs", "fn main() {}\n"),
            ("work/docs/manifestdocs.yaml", "name: cookbook\n"),
            ("work/docs/cookbook.md", &node("fd-0a0001", "Cookbook")),
            ("work/docs/search/search.md", &node("fd-7c2e41", "Search")),
        ],
    );
    git(&fixture, "work/web", &["init", "-q"]);
    git(&fixture, "", &["init", "-q", "--bare", "origin.git"]);
    let main = commit(&fixture, "Add the cookbook");
    git(
        &fixture,
        "work/docs",
        &["remote", "add", "origin", "../../origin.git"],
    );
    git(&fixture, "work/docs", &["push", "-q", "origin", "main"]);
    git(
        &fixture,
        "work/docs",
        &["remote", "set-head", "origin", "main"],
    );
    git(
        &fixture,
        "work/docs",
        &["checkout", "-q", "-b", "feature/x"],
    );
    retitle(&fixture, "Search, feature");
    let feature = commit(&fixture, "Retitle search");
    retitle(&fixture, "Search, edited");
    (fixture, main, feature)
}

/// Commits everything in the docs clone, returning the short commit.
fn commit(fixture: &Fixture, message: &str) -> String {
    let docs = "work/docs";
    if !fixture.path("work/docs/.git/HEAD").exists() {
        git(fixture, docs, &["init", "-q", "-b", "main"]);
    }
    git(fixture, docs, &["add", "-A"]);
    git(fixture, docs, &["commit", "-q", "-m", message]);
    git(fixture, docs, &["rev-parse", "--short", "HEAD"])
}

fn retitle(fixture: &Fixture, title: &str) {
    let text = node("fd-7c2e41", title);
    fs::write(fixture.path("work/docs/search/search.md"), text).unwrap();
}

/// Runs `git` in `folder`, with its own home folder so no user config applies.
pub fn git(fixture: &Fixture, folder: &str, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(["-c", "user.name=Sam", "-c", "user.email=sam@example.com"])
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .current_dir(fixture.path(folder))
        .env("HOME", fixture.path(""))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_DATE", "2026-10-27T12:00:00Z")
        .env("GIT_COMMITTER_DATE", "2026-10-27T12:00:00Z")
        .output()
        .expect("git runs");
    assert!(output.status.success(), "git {args:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// `tree`'s line for the Search node with `title`, from `work/web`.
fn search_line(title: &str) -> String {
    format!("  {title} (fd-7c2e41, draft)  ../docs/search/search.md")
}

/// `check`, `tree` and `show` are for authors, so they see uncommitted edits from any repo.
#[test]
fn check_tree_and_show_read_the_working_tree_through_a_pointer() {
    let (fixture, _, _) = workspace("source-pointer-working-tree");
    let broken = format!("{}[gone](gone.md)\n", node("fd-7c2e41", "Search, edited"));
    fs::write(fixture.path("work/docs/search/search.md"), broken).unwrap();

    let tree = fixture.run("work/web/src", &["tree"]);
    let show = fixture.run("work/web/src", &["show", "fd-7c2e41"]);
    let check = fixture.run("work/web/src", &["check"]);

    assert_eq!(
        tree.stdout,
        "Cookbook (fd-0a0001, draft)  ../../docs/cookbook.md\n  \
         Search, edited (fd-7c2e41, draft)  ../../docs/search/search.md\n"
    );
    assert!(
        show.stdout
            .starts_with("Search, edited (fd-7c2e41, draft)\n"),
        "{}{}",
        show.stdout,
        show.stderr
    );
    assert_eq!(
        check.stdout,
        "../../docs/search/search.md:9: error: broken link `gone.md`\n\n2 nodes: 1 error\n"
    );
    assert_eq!(check.code, 1);
}

#[test]
fn reads_the_working_tree_inside_the_docs_clone() {
    let (fixture, _, _) = workspace("source-working-tree");

    let run = fixture.run("work/docs", &["tree"]);

    assert_eq!(
        run.stdout,
        "Cookbook (fd-0a0001, draft)  cookbook.md\n  \
         Search, edited (fd-7c2e41, draft)  search/search.md\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn tree_reads_the_ref_given() {
    let (fixture, _, _) = workspace("source-ref-tree");

    let run = fixture.run("work/web", &["tree", "--ref", "feature/x"]);

    assert_eq!(
        run.stdout.lines().nth(1),
        Some(&*search_line("Search, feature"))
    );
    assert_eq!(run.code, 0, "{}", run.stderr);
}

#[test]
fn show_reads_the_ref_given_inside_the_docs_clone() {
    let (fixture, _, _) = workspace("source-ref-show");

    let run = fixture.run("work/docs", &["show", "fd-7c2e41", "--ref", "main"]);

    assert!(
        run.stdout
            .starts_with("Search (fd-7c2e41, draft)\nFile: search/search.md\n"),
        "{}{}",
        run.stdout,
        run.stderr
    );
    assert_eq!(run.code, 0);
}

#[test]
fn check_reads_the_ref_given() {
    let (fixture, _, _) = workspace("source-ref-check");
    git(&fixture, "work/docs", &["checkout", "-q", "-f", "main"]);
    let broken = format!("{}[gone](gone.md)\n", node("fd-7c2e41", "Search"));
    fs::write(fixture.path("work/docs/search/search.md"), broken).unwrap();
    git(
        &fixture,
        "work/docs",
        &["commit", "-q", "-am", "Break a link"],
    );
    git(&fixture, "work/docs", &["branch", "-q", "broken"]);
    git(&fixture, "work/docs", &["reset", "-q", "--hard", "HEAD~"]);

    let run = fixture.run("work/docs", &["check", "--ref", "broken"]);

    assert_eq!(
        run.stdout,
        "search/search.md:9: error: broken link `gone.md`\n\n2 nodes: 1 error\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn context_reads_the_ref_given() {
    let (fixture, _, feature) = workspace("source-ref-context");

    let run = fixture.run("work/web", &["context", "fd-7c2e41", "--ref", "feature/x"]);

    assert!(
        run.stdout.starts_with(&format!(
            "# Context: fd-7c2e41 (docs feature/x @ {feature}, 2026-10-27)\n\n\
             ## Feature: Search, feature (fd-7c2e41, draft)\n"
        )),
        "{}{}",
        run.stdout,
        run.stderr
    );
    assert_eq!(run.code, 0);
}

#[test]
fn fails_on_a_ref_the_docs_clone_doesnt_have() {
    let (fixture, _, _) = workspace("source-ref-missing");

    let run = fixture.run("work/web", &["tree", "--ref", "nope"]);

    assert_eq!(
        run.stderr,
        "error: no commit `nope` in the docs clone `../docs`\n"
    );
    assert_eq!(run.code, 2);
}

#[test]
fn fails_with_the_set_head_command_when_origin_head_is_unset() {
    let (fixture, _, _) = workspace("source-no-origin-head");
    git(
        &fixture,
        "work/docs",
        &["remote", "set-head", "origin", "-d"],
    );

    let run = fixture.run("work/web", &["context", "fd-7c2e41"]);

    assert_eq!(
        run.stderr,
        "error: the docs clone's `origin/HEAD` isn't set, so its default branch is unknown; \
         set it with `git -C ../docs remote set-head origin --auto`\n"
    );
    assert_eq!(run.code, 2);
}

/// A code repo whose pointer leads to a tree in no git repo.
fn outside_git(name: &str) -> Fixture {
    let fixture = Fixture::new(
        name,
        &[
            POINTER,
            ("work/web/src/main.rs", "fn main() {}\n"),
            ("work/docs/manifestdocs.yaml", "name: cookbook\n"),
            ("work/docs/cookbook.md", &node("fd-0a0001", "Cookbook")),
        ],
    );
    git(&fixture, "work/web", &["init", "-q"]);
    fixture
}

#[test]
fn reads_the_files_of_a_tree_outside_git() {
    let fixture = outside_git("source-outside-git");

    let run = fixture.run("work/web", &["tree"]);

    assert_eq!(
        run.stdout,
        "Cookbook (fd-0a0001, draft)  ../docs/cookbook.md\n"
    );
    assert_eq!(run.code, 0, "{}", run.stderr);
}

#[test]
fn rejects_a_ref_for_a_tree_outside_git() {
    let fixture = outside_git("source-outside-git-ref");

    let run = fixture.run("work/web", &["tree", "--ref", "main"]);

    assert_eq!(
        run.stderr,
        "error: `--ref` needs a docs clone, and the tree `../docs` isn't in a git repo\n"
    );
    assert_eq!(run.code, 2);
}

#[test]
fn lists_tracked_files_that_gitignore_matches() {
    let (fixture, _, _) = workspace("source-tracked-ignored");
    git(&fixture, "work/docs", &["checkout", "-q", "-f", "main"]);
    let linked = format!("{}[table](table.md)\n", node("fd-7c2e41", "Search"));
    fs::write(fixture.path("work/docs/search/search.md"), linked).unwrap();
    fs::write(fixture.path("work/docs/search/table.md"), "# Table\n").unwrap();
    fs::write(fixture.path("work/docs/.gitignore"), "table.md\n").unwrap();
    git(&fixture, "work/docs", &["add", "-A"]);
    git(&fixture, "work/docs", &["add", "-f", "search/table.md"]);
    git(
        &fixture,
        "work/docs",
        &["commit", "-q", "-m", "Add a table"],
    );
    git(&fixture, "work/docs", &["push", "-q", "origin", "main"]);

    let working_tree = fixture.run("work/docs", &["check"]);
    let commit = fixture.run("work/web", &["check", "--ref", "main"]);

    assert_eq!(working_tree.stdout, "2 nodes: no problems\n");
    assert_eq!(commit.stdout, "2 nodes: no problems\n");
}

#[test]
fn leaves_out_a_tracked_file_deleted_from_the_working_tree() {
    let (fixture, _, _) = workspace("source-tracked-deleted");
    git(&fixture, "work/docs", &["checkout", "-q", "-f", "main"]);
    let linked = format!("{}[table](table.md)\n", node("fd-7c2e41", "Search"));
    fs::write(fixture.path("work/docs/search/search.md"), linked).unwrap();
    fs::write(fixture.path("work/docs/search/table.md"), "# Table\n").unwrap();
    commit(&fixture, "Add a table");
    fs::remove_file(fixture.path("work/docs/search/table.md")).unwrap();

    let run = fixture.run("work/docs", &["check"]);

    assert_eq!(
        run.stdout,
        "search/search.md:9: error: broken link `table.md`\n\n2 nodes: 1 error\n"
    );
    assert_eq!(run.code, 1);
}

/// `context` feeds agents, so from another repo it reads only the merged default branch.
#[test]
fn context_reads_origin_head_through_a_pointer() {
    let (fixture, main, _) = workspace("source-header");

    let run = fixture.run("work/web", &["context", "fd-7c2e41"]);

    assert!(
        run.stdout.starts_with(&format!(
            "# Context: fd-7c2e41 (docs origin/main @ {main}, 2026-10-27)\n\n\
             ## Feature: Search (fd-7c2e41, draft)\n"
        )),
        "{}{}",
        run.stdout,
        run.stderr
    );
    assert_eq!(run.code, 0);
}

#[test]
fn writes_the_context_to_the_out_file() {
    let (fixture, _, _) = workspace("source-out");
    fs::create_dir_all(fixture.path("work/web/.context")).unwrap();

    let printed = fixture.run("work/web", &["context", "fd-7c2e41"]);
    let run = fixture.run(
        "work/web",
        &["context", "fd-7c2e41", "--out", ".context/context.md"],
    );

    let written = fs::read_to_string(fixture.path("work/web/.context/context.md")).unwrap();
    assert_eq!(written, printed.stdout);
    assert_eq!(run.stdout, "");
    assert_eq!(run.stderr, "Wrote .context/context.md\n");
    assert_eq!(run.code, 0);
}

#[test]
fn creates_the_out_files_folders() {
    let (fixture, _, _) = workspace("source-out-folders");

    let run = fixture.run(
        "work/web",
        &["context", "fd-7c2e41", "--out", ".context/run-1/context.md"],
    );

    let written = fs::read_to_string(fixture.path("work/web/.context/run-1/context.md"));
    assert!(
        written
            .unwrap()
            .starts_with("# Context: fd-7c2e41 (docs origin/main @ ")
    );
    assert_eq!(run.stderr, "Wrote .context/run-1/context.md\n");
    assert_eq!(run.code, 0);
}

#[test]
fn finds_a_path_that_is_only_in_the_version_read() {
    let (fixture, _, _) = workspace("source-path-in-commit");
    git(&fixture, "work/docs", &["checkout", "-q", "-f", "main"]);
    let filters = node("fd-7c2e99", "Filters");
    fs::create_dir_all(fixture.path("work/docs/search/filters")).unwrap();
    fs::write(fixture.path("work/docs/search/filters/filters.md"), filters).unwrap();
    commit(&fixture, "Add filters");
    git(&fixture, "work/docs", &["push", "-q", "origin", "main"]);
    git(&fixture, "work/docs", &["checkout", "-q", "feature/x"]);
    assert!(!fixture.path("work/docs/search/filters").exists());

    let filters = "../docs/search/filters";
    let show = fixture.run("work/web", &["show", filters, "--ref", "main"]);
    let file = "../docs/search/filters/filters.md";
    let tree = fixture.run("work/web", &["tree", file, "--ref", "main"]);
    let context = fixture.run("work/web", &["context", filters]);

    assert!(
        show.stdout.starts_with("Filters (fd-7c2e99, draft)\n"),
        "{}",
        show.stderr
    );
    assert_eq!(
        tree.stdout,
        "Filters (fd-7c2e99, draft)  ../docs/search/filters/filters.md\n"
    );
    assert!(
        context.stdout.contains("## Feature: Filters (fd-7c2e99"),
        "{}",
        context.stderr
    );
}

#[test]
fn doesnt_find_a_path_that_is_only_on_disk() {
    let (fixture, _, _) = workspace("source-path-on-disk");
    let drafts = node("fd-0d0001", "Drafts");
    fs::create_dir_all(fixture.path("work/docs/drafts")).unwrap();
    fs::write(fixture.path("work/docs/drafts/drafts.md"), drafts).unwrap();

    let through_pointer = fixture.run("work/web", &["context", "../docs/drafts"]);
    let with_ref = fixture.run("work/docs", &["tree", "drafts", "--ref", "main"]);
    let working_tree = fixture.run("work/docs", &["tree", "drafts"]);

    assert_eq!(
        through_pointer.stderr,
        "error: no file or folder `../docs/drafts`\n"
    );
    assert_eq!(through_pointer.code, 2);
    assert_eq!(with_ref.stderr, "error: no file or folder `drafts`\n");
    assert_eq!(with_ref.code, 2);
    assert_eq!(
        working_tree.stdout,
        "Drafts (fd-0d0001, draft)  drafts/drafts.md\n"
    );
}

#[test]
fn rejects_a_ref_that_starts_with_a_dash() {
    let (fixture, _, _) = workspace("source-ref-dash");

    let run = fixture.run("work/docs", &["tree", "--ref=--output=leak"]);

    assert_eq!(
        run.stderr,
        "error: `--ref` can't start with `-`: `--output=leak`\n"
    );
    assert_eq!(run.code, 2);
}

#[test]
fn fails_when_it_cant_write_the_out_file() {
    let (fixture, _, _) = workspace("source-out-fails");

    let out = "src/main.rs/context.md";
    let run = fixture.run("work/web", &["context", "fd-7c2e41", "--out", out]);

    assert!(
        run.stderr
            .starts_with("error: can't write src/main.rs/context.md: "),
        "{}",
        run.stderr
    );
    assert_eq!(run.stdout, "");
    assert_eq!(run.code, 2);
}

/// With `log.showSignature`, git prints signature lines before a commit's formatted line.
#[test]
fn reads_signed_commits_when_git_shows_signatures() {
    let (fixture, _, _) = workspace("source-signed");
    let key = fixture.path("key");
    let keygen = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", ""])
        .arg("-f")
        .arg(&key)
        .output()
        .expect("ssh-keygen runs");
    assert!(keygen.status.success(), "{keygen:?}");
    let docs = "work/docs";
    git(&fixture, docs, &["config", "log.showSignature", "true"]);
    git(&fixture, docs, &["config", "gpg.format", "ssh"]);
    let key = key.to_str().unwrap();
    git(&fixture, docs, &["config", "user.signingkey", key]);
    git(&fixture, docs, &["checkout", "-q", "-f", "main"]);
    retitle(&fixture, "Search, signed");
    git(&fixture, docs, &["commit", "-q", "-S", "-am", "Sign"]);
    git(&fixture, docs, &["push", "-q", "origin", "main"]);
    let signed = git(&fixture, docs, &["rev-parse", "--short", "HEAD"]);

    let commit = fixture.run("work/web", &["context", "fd-7c2e41"]);
    let working_tree = fixture.run("work/docs", &["context", "fd-7c2e41"]);

    let header = |run: &Run| run.stdout.lines().next().unwrap_or_default().to_owned();
    let version = |rest: &str| format!("# Context: fd-7c2e41 (docs {rest} @ {signed}, 2026-10-27)");
    assert_eq!(header(&commit), version("origin/main"), "{}", commit.stderr);
    assert_eq!(header(&working_tree), version("main"));
}

/// Runs `manifestdocs` as `Fixture::run` does, with `env` set too.
fn run_with(fixture: &Fixture, folder: &str, args: &[&str], env: &[(&str, &str)]) -> Run {
    let home = fixture.path("");
    let output = Command::new(env!("CARGO_BIN_EXE_manifestdocs"))
        .args(args)
        .current_dir(fixture.path(folder))
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("GIT_CEILING_DIRECTORIES", home.parent().unwrap())
        .envs(env.iter().copied())
        .output()
        .expect("the binary runs");
    Run {
        code: output.status.code().expect("the binary exits with a code"),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}

/// A hook in the code repo can run the tool with `GIT_DIR` set to the code repo's.
#[test]
fn ignores_a_git_dir_set_for_another_repo() {
    let (fixture, main, _) = workspace("source-git-dir");
    let git_dir = fixture.path("work/web/.git");

    let run = run_with(
        &fixture,
        "work/web",
        &["context", "fd-7c2e41"],
        &[("GIT_DIR", git_dir.to_str().unwrap())],
    );

    assert!(
        run.stdout.starts_with(&format!(
            "# Context: fd-7c2e41 (docs origin/main @ {main}, 2026-10-27)\n"
        )),
        "{}{}",
        run.stdout,
        run.stderr
    );
}

/// A partial clone can lack an object, and the tool never fetches it.
#[test]
fn names_the_commit_when_the_clone_lacks_a_file() {
    let (fixture, main, _) = workspace("source-missing-blob");
    let blob = git(
        &fixture,
        "work/docs",
        &["rev-parse", "main:search/search.md"],
    );
    let object = format!("work/docs/.git/objects/{}/{}", &blob[..2], &blob[2..]);
    fs::remove_file(fixture.path(&object)).unwrap();

    let run = fixture.run("work/web", &["context", "fd-7c2e41"]);

    assert_eq!(
        run.stderr,
        format!(
            "error: can't read ../docs/search/search.md in `origin/main` @ {main}: \
             the clone doesn't have blob {blob}, and manifestdocs never fetches\n"
        )
    );
    assert_eq!(run.code, 2);
}

#[test]
fn fails_when_git_cant_run_for_a_docs_clone() {
    let (fixture, _, _) = workspace("source-no-git");

    let run = run_with(
        &fixture,
        "work/docs",
        &["tree"],
        &[("PATH", "/nonexistent")],
    );

    assert!(
        run.stderr
            .starts_with("error: can't read the docs clone `.`: can't run `git`: "),
        "{}",
        run.stderr
    );
    assert_eq!(run.code, 2);
}
