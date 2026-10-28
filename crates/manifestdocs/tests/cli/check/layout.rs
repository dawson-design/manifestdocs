use crate::support::{Fixture, node};

const CONFIG: (&str, &str) = ("manifestdocs.yaml", "name: demo\n");

#[test]
fn passes_a_tree_with_a_root_node_and_features() {
    let root = node("fd-000001", "Demo");
    let recipes = node("fd-000002", "Recipes");
    let fixture = Fixture::new(
        "clean",
        &[
            CONFIG,
            ("demo.md", &root),
            ("recipes/recipes.md", &recipes),
            ("recipes/notes.md", "# Notes\n"),
        ],
    );

    let run = fixture.check("");

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(run.code, 0);
}

#[test]
fn warns_when_the_root_node_is_missing() {
    let recipes = node("fd-000002", "Recipes");
    let fixture = Fixture::new("no-root", &[CONFIG, ("recipes/recipes.md", &recipes)]);

    let run = fixture.check("");

    assert_eq!(
        run.stdout,
        "manifestdocs.yaml: warning: no root node `demo.md`\n\n1 node: 1 warning\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn skips_a_nested_tree_with_a_warning() {
    let root = node("fd-000001", "Demo");
    let vendor = node("fd-000003", "Vendor");
    let parts = node("fd-000004", "Parts");
    let fixture = Fixture::new(
        "nested",
        &[
            CONFIG,
            ("demo.md", &root),
            ("vendor/manifestdocs.yaml", "name: vendor\n"),
            ("vendor/vendor.md", &vendor),
            ("vendor/parts/parts.md", &parts),
        ],
    );

    let run = fixture.check("");

    assert_eq!(
        run.stdout,
        "vendor/: warning: skipped nested tree\n\n1 node: 1 warning\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn never_treats_a_decisions_folder_as_a_node() {
    let root = node("fd-000001", "Demo");
    let decisions = node("fd-000005", "Decisions");
    let fixture = Fixture::new(
        "decisions",
        &[
            CONFIG,
            ("demo.md", &root),
            ("decisions/decisions.md", &decisions),
        ],
    );

    let run = fixture.check("");

    assert_eq!(run.stdout, "1 node: no problems\n");
    assert_eq!(run.code, 0);
}

#[test]
fn finds_the_tree_from_a_subfolder_and_names_paths_from_there() {
    let recipes = node("fd-000002", "Recipes");
    let fixture = Fixture::new("subfolder", &[CONFIG, ("recipes/recipes.md", &recipes)]);

    let run = fixture.check("recipes");

    assert_eq!(
        run.stdout,
        "../manifestdocs.yaml: warning: no root node `demo.md`\n\n1 node: 1 warning\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn fails_when_no_tree_is_found() {
    let fixture = Fixture::new("no-tree", &[("README.md", "# Readme\n")]);

    let run = fixture.check("");

    assert_eq!(
        run.stderr,
        "error: no manifestdocs.yaml in this folder or any folder above it, up to the repo root, and no .manifestdocs/config.yaml at or above it\n"
    );
    assert_eq!(run.stdout, "");
    assert_eq!(run.code, 2);
}

#[test]
fn stops_looking_for_the_tree_at_the_repo_root() {
    let fixture = Fixture::new(
        "repo-boundary",
        &[
            CONFIG,
            ("inner/.git/HEAD", "ref: refs/heads/main\n"),
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
fn fails_on_an_invalid_config_with_its_line() {
    let fixture = Fixture::new(
        "bad-config",
        &[("manifestdocs.yaml", "name: demo\nstatues: [draft]\n")],
    );

    let run = fixture.check("");

    assert_eq!(
        run.stderr,
        "error: manifestdocs.yaml:2: unknown field `statues`, expected one of name, github, decisions, statuses\n"
    );
    assert_eq!(run.stdout, "");
    assert_eq!(run.code, 2);
}
