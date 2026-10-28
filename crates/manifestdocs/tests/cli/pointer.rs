//! Finding the tree through a `.manifestdocs/config.yaml` pointer.

use crate::support::{Fixture, node};

const WEB_POINTER: &str = "work/cookbook/cookbook-web/.manifestdocs/config.yaml";
const COOKBOOK_POINTER: &str = "work/cookbook/.manifestdocs/config.yaml";

/// A docs repo and a code repo with its own `docs/` tree side by side, plus `extra` files.
fn pointer_fixture(name: &str, extra: &[(&str, &str)]) -> Fixture {
    let docs_root = format!("{}[x](gone.md)\n", node("fd-000001", "Cookbook"));
    let notes_root = node("fd-000002", "Notes");
    let web_root = node("fd-000003", "Web");
    let mut files = vec![
        ("work/cookbook/cookbook-docs/.git/HEAD", ""),
        (
            "work/cookbook/cookbook-docs/manifestdocs.yaml",
            "name: cookbook\n",
        ),
        ("work/cookbook/cookbook-docs/cookbook.md", &docs_root),
        ("work/cookbook/cookbook-web/.git/HEAD", ""),
        ("work/cookbook/cookbook-web/src/main.rs", "fn main() {}\n"),
        (
            "work/cookbook/cookbook-web/docs/manifestdocs.yaml",
            "name: web\n",
        ),
        ("work/cookbook/cookbook-web/docs/web.md", &web_root),
        ("work/notes/manifestdocs.yaml", "name: notes\n"),
        ("work/notes/notes.md", &notes_root),
        ("work/scratch/todo.txt", "x\n"),
    ];
    files.extend_from_slice(extra);
    Fixture::new(name, &files)
}

#[test]
fn finds_the_repos_own_tree_from_anywhere_in_the_repo() {
    let fixture = pointer_fixture("pointer-single-repo", &[(WEB_POINTER, "tree: docs\n")]);

    let check = fixture.check("work/cookbook/cookbook-web");
    let from_root = fixture.run("work/cookbook/cookbook-web", &["tree"]);
    let from_src = fixture.run("work/cookbook/cookbook-web/src", &["tree"]);

    assert_eq!(check.stdout, "1 node: no problems\n");
    assert_eq!(check.code, 0);
    assert_eq!(from_root.stdout, "Web (fd-000003, draft)  docs/web.md\n");
    assert_eq!(from_src.stdout, "Web (fd-000003, draft)  ../docs/web.md\n");
}

#[test]
fn finds_a_docs_repo_beside_the_code_repo() {
    let fixture = pointer_fixture(
        "pointer-multi-repo",
        &[(COOKBOOK_POINTER, "tree: cookbook-docs\n")],
    );

    let check = fixture.check("work/cookbook/cookbook-web/src");
    let tree = fixture.run("work/cookbook/cookbook-web/src", &["tree"]);
    let show = fixture.run(
        "work/cookbook/cookbook-web/src",
        &["show", "../../cookbook-docs"],
    );

    assert_eq!(
        check.stdout,
        "../../cookbook-docs/cookbook.md:9: error: broken link `gone.md`\n\n1 node: 1 error\n"
    );
    assert_eq!(check.code, 1);
    assert_eq!(
        tree.stdout,
        "Cookbook (fd-000001, draft)  ../../cookbook-docs/cookbook.md\n"
    );
    assert!(
        show.stdout
            .starts_with("Cookbook (fd-000001, draft)\nFile: ../../cookbook-docs/cookbook.md\n"),
        "{}",
        show.stdout
    );
}

#[test]
fn follows_a_tree_path_that_climbs_out_of_the_folder() {
    let fixture = pointer_fixture(
        "pointer-climbs",
        &[(WEB_POINTER, "tree: ../cookbook-docs\n")],
    );

    let run = fixture.run("work/cookbook/cookbook-web/src", &["tree"]);

    assert_eq!(
        run.stdout,
        "Cookbook (fd-000001, draft)  ../../cookbook-docs/cookbook.md\n"
    );
}

#[test]
fn prefers_a_tree_found_by_walking_up() {
    let fixture = pointer_fixture(
        "pointer-walk-up",
        &[(COOKBOOK_POINTER, "tree: cookbook-docs\n")],
    );

    let run = fixture.run("work/cookbook/cookbook-web/docs", &["tree"]);

    assert_eq!(run.stdout, "Web (fd-000003, draft)  web.md\n");
}

#[test]
fn uses_the_nearest_pointer_at_or_above_the_folder() {
    let fixture = pointer_fixture(
        "pointer-nearest",
        &[
            (WEB_POINTER, "tree: docs\n"),
            (COOKBOOK_POINTER, "tree: cookbook-docs\n"),
            (".manifestdocs/config.yaml", "tree: work/notes\n"),
        ],
    );

    let web = fixture.run("work/cookbook/cookbook-web/src", &["tree"]);
    let cookbook = fixture.run("work/cookbook", &["tree"]);
    let notes = fixture.run("work/scratch", &["tree"]);

    assert_eq!(web.stdout, "Web (fd-000003, draft)  ../docs/web.md\n");
    assert_eq!(
        cookbook.stdout,
        "Cookbook (fd-000001, draft)  cookbook-docs/cookbook.md\n"
    );
    assert_eq!(
        notes.stdout,
        "Notes (fd-000002, draft)  ../notes/notes.md\n"
    );
}

#[test]
fn skips_a_manifestdocs_folder_without_a_pointer() {
    let fixture = pointer_fixture(
        "pointer-drafts-only",
        &[
            (
                "work/cookbook/cookbook-web/.manifestdocs/decisions/2026-10-28-engine.md",
                "draft\n",
            ),
            (COOKBOOK_POINTER, "tree: cookbook-docs\n"),
        ],
    );

    let run = fixture.run("work/cookbook/cookbook-web/src", &["tree"]);

    assert_eq!(
        run.stdout,
        "Cookbook (fd-000001, draft)  ../../cookbook-docs/cookbook.md\n"
    );
}

#[test]
fn stops_when_the_pointer_cant_be_used() {
    let cases = [
        (
            "pointer-absolute",
            "tree: /srv/docs\n",
            "error: ../.manifestdocs/config.yaml:1: `tree` `/srv/docs` must be a relative path\n",
        ),
        (
            "pointer-unknown-key",
            "tree: cookbook-docs\nname: x\n",
            "error: ../.manifestdocs/config.yaml:2: unknown field `name`, expected one of tree\n",
        ),
        (
            "pointer-no-tree",
            "tree: scratch\n",
            "error: ../.manifestdocs/config.yaml: tree `scratch` has no manifestdocs.yaml\n",
        ),
    ];
    for (name, pointer, expected) in cases {
        let fixture = pointer_fixture(name, &[(COOKBOOK_POINTER, pointer)]);

        let run = fixture.check("work/cookbook/cookbook-web");

        assert_eq!(run.stderr, expected, "{name}");
        assert_eq!(run.stdout, "", "{name}");
        assert_eq!(run.code, 2, "{name}");
    }
}
