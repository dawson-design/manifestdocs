use std::fs;

use crate::support::{Fixture, node};

#[test]
fn checks_a_tree_below_the_repo_root_and_ignores_files_beside_it() {
    let root = node("fd-000001", "Demo");
    let recipes = node("fd-000002", "Recipes");
    let other = node("fd-000003", "Other");
    let fixture = Fixture::new(
        "below-root",
        &[
            ("README.md", "# Readme\n"),
            ("docs/manifestdocs.yaml", "name: demo\n"),
            ("docs/demo.md", &root),
            ("docs/recipes/recipes.md", &recipes),
            ("other/other.md", &other),
        ],
    );

    let run = fixture.check("docs/recipes");

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(run.code, 0);
}

#[test]
fn reads_what_github_shows_dotfiles_included_and_gitignored_files_left_out() {
    let root = node("fd-000001", "Demo");
    let recipes = node("fd-000002", "Recipes");
    let drafts = node("fd-000004", "Drafts");
    let fixture = Fixture::new(
        "ignore-rules",
        &[
            ("manifestdocs.yaml", "name: demo\n"),
            ("demo.md", &root),
            ("recipes/recipes.md", &recipes),
            ("drafts/drafts.md", &drafts),
            (".gitignore", "drafts/\n"),
            (".ignore", "recipes/\n"),
            (".vendor/manifestdocs.yaml", "name: vendor\n"),
        ],
    );

    let run = fixture.check("");

    assert_eq!(
        run.stdout,
        ".vendor/: warning: skipped nested tree\n\n2 nodes: 1 warning\n"
    );
}

#[test]
fn skips_a_nested_repo() {
    let root = node("fd-000001", "Demo");
    let sub = node("fd-000005", "Sub");
    let fixture = Fixture::new(
        "nested-repo",
        &[
            ("manifestdocs.yaml", "name: demo\n"),
            ("demo.md", &root),
            ("sub/.git/HEAD", "ref: refs/heads/main\n"),
            ("sub/sub.md", &sub),
        ],
    );

    let run = fixture.check("");

    assert_eq!(run.stdout, "1 node: no problems\n");
}

#[test]
fn stops_looking_for_the_tree_at_a_worktree_git_file() {
    let fixture = Fixture::new(
        "worktree",
        &[
            ("manifestdocs.yaml", "name: demo\n"),
            ("inner/.git", "gitdir: ../.git/worktrees/inner\n"),
            ("inner/notes.md", "# Notes\n"),
        ],
    );

    let run = fixture.check("inner");

    assert!(
        run.stderr.starts_with("error: no manifestdocs.yaml"),
        "{}",
        run.stderr
    );
    assert_eq!(run.code, 2);
}

#[test]
fn fails_on_markdown_that_isnt_utf8() {
    let fixture = Fixture::new("not-utf8", &[("manifestdocs.yaml", "name: demo\n")]);
    fs::write(fixture.path("bad.md"), [0xff, 0xfe, b'#']).expect("write a fixture file");

    let run = fixture.check("");

    assert!(
        run.stderr
            .starts_with("error: can't read bad.md as UTF-8 text"),
        "{}",
        run.stderr
    );
    assert_eq!(run.stdout, "");
    assert_eq!(run.code, 2);
}

#[test]
fn ignores_markdown_inside_a_skipped_nested_tree() {
    let root = node("fd-000001", "Demo");
    let fixture = Fixture::new(
        "nested-not-utf8",
        &[
            ("manifestdocs.yaml", "name: demo\n"),
            ("demo.md", &root),
            ("vendor/manifestdocs.yaml", "name: vendor\n"),
        ],
    );
    fs::write(fixture.path("vendor/bad.md"), [0xff, 0xfe]).expect("write a fixture file");

    let run = fixture.check("");

    assert_eq!(
        run.stdout,
        "vendor/: warning: skipped nested tree\n\n1 node: 1 warning\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn checks_the_tree_that_contains_a_given_path() {
    let broken =
        "---\nid: fd-000002\ntitle: Search\nstatus: draft\nsummary: \"Search summary.\"\n---\n";
    let fixture = Fixture::new(
        "check-path",
        &[
            (".manifestdocs/config.yaml", "tree: docs\n"),
            ("README.md", "# Cookbook\n"),
            ("docs/manifestdocs.yaml", "name: cookbook\n"),
            ("docs/cookbook.md", &node("fd-000001", "Cookbook")),
            ("docs/search/search.md", broken),
            ("notes/manifestdocs.yaml", "name: notes\n"),
            ("notes/notes.md", &node("fd-000003", "Notes")),
        ],
    );

    let in_docs = fixture.run("", &["check", "docs/search/search.md"]);
    let in_notes = fixture.run("", &["check", "notes"]);
    let outside = fixture.run("", &["check", "README.md"]);
    let here = fixture.run("", &["check", "."]);
    let plain = fixture.run("", &["check"]);
    let missing = fixture.run("", &["check", "gone.md"]);

    assert_eq!(
        in_docs.stdout,
        "docs/search/search.md:3: error: `title` must be double-quoted\n\n2 nodes: 1 error\n"
    );
    assert_eq!(in_docs.code, 1);
    assert_eq!(in_notes.stdout, "1 node: no problems\n");
    assert_eq!(
        outside.stderr,
        "error: `README.md` isn't in a tree: the nearest tree is `docs`\n"
    );
    assert_eq!(outside.code, 2);
    // A folder is checked as running `check` in it would be, through the pointer.
    assert_eq!(here.stdout, plain.stdout);
    assert_eq!(here.code, 1);
    assert_eq!(missing.stderr, "error: no file or folder `gone.md`\n");
    assert_eq!(missing.code, 2);
}
