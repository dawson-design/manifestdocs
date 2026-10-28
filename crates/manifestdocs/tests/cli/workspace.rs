//! Finding the tree through the workspaces in the user config.

use crate::support::{Fixture, node};

const USER_CONFIG: &str = ".config/manifestdocs/config.yaml";

/// A home folder with a docs repo and a code repo side by side, and `config` as the user config.
fn workspace_fixture(name: &str, config: &str) -> Fixture {
    let docs_root = format!("{}[x](gone.md)\n", node("fd-000001", "Cookbook"));
    let notes_root = node("fd-000002", "Notes");
    Fixture::new(
        name,
        &[
            (USER_CONFIG, config),
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
            ("work/notes/manifestdocs.yaml", "name: notes\n"),
            ("work/notes/notes.md", &notes_root),
            ("work/scratch/todo.txt", "x\n"),
        ],
    )
}

const COOKBOOK: &str =
    "workspaces:\n  - path: ~/work/cookbook\n    tree: ~/work/cookbook/cookbook-docs\n";

#[test]
fn finds_the_tree_from_a_code_repo_through_the_user_config() {
    let fixture = workspace_fixture("workspace-code-repo", COOKBOOK);

    let run = fixture.check("work/cookbook/cookbook-web/src");

    assert_eq!(
        run.stdout,
        "../../cookbook-docs/cookbook.md:9: error: broken link `gone.md`\n\n1 node: 1 error\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn uses_the_longest_workspace_that_contains_the_folder() {
    let config = "workspaces:\n  - path: ~/work\n    tree: ~/work/notes\n  - path: ~/work/cookbook\n    tree: ~/work/cookbook/cookbook-docs\n";
    let fixture = workspace_fixture("workspace-longest", config);

    let cookbook = fixture.check("work/cookbook/cookbook-web");
    let notes = fixture.check("work/scratch");

    assert!(
        cookbook.stdout.ends_with("1 node: 1 error\n"),
        "{}",
        cookbook.stdout
    );
    assert_eq!(notes.stdout, "1 node: no problems\n");
}

#[test]
fn prefers_a_tree_found_by_walking_up() {
    let fixture = workspace_fixture("workspace-walk-up", COOKBOOK);

    let run = fixture.check("work/cookbook/cookbook-web/docs");

    assert_eq!(
        run.stdout,
        "manifestdocs.yaml: warning: no root node `web.md`\n\n0 nodes: 1 warning\n"
    );
}

#[test]
fn stops_when_the_user_config_cant_be_used() {
    let cases = [
        (
            "workspace-relative-path",
            "workspaces:\n  - path: work/cookbook\n    tree: ~/work/cookbook/cookbook-docs\n",
            "error: ~/.config/manifestdocs/config.yaml:2: workspace `path` `work/cookbook` must be absolute or start with `~/`\n",
        ),
        (
            "workspace-no-tree",
            "workspaces:\n  - path: ~/work/cookbook\n    tree: ~/work/cookbook\n",
            "error: ~/.config/manifestdocs/config.yaml: workspace tree `~/work/cookbook` has no manifestdocs.yaml\n",
        ),
        (
            "workspace-no-match",
            "workspaces:\n  - path: ~/elsewhere\n    tree: ~/work/notes\n",
            "error: no manifestdocs.yaml in this folder or any folder above it, up to the repo root, or in a workspace in ~/.config/manifestdocs/config.yaml\n",
        ),
    ];
    for (name, config, expected) in cases {
        let fixture = workspace_fixture(name, config);

        let run = fixture.check("work/cookbook/cookbook-web");

        assert_eq!(run.stderr, expected, "{name}");
        assert_eq!(run.stdout, "", "{name}");
        assert_eq!(run.code, 2, "{name}");
    }
}
