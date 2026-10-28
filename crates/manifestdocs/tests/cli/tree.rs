//! `manifestdocs tree`.

use crate::support::{Fixture, node};

/// A cookbook tree with nested nodes, a pass-through folder, a broken node, a superseded node,
/// a supporting doc, a decision record and a nested tree.
fn cookbook(name: &str, with_root: bool) -> Fixture {
    let root = node("fd-000001", "Cookbook");
    let recipes = node("fd-000002", "Recipes");
    let knives = node("fd-000006", "Knives");
    let vendor = node("fd-000007", "Vendor");
    let search = "---\nid: fd-000003\ntitle: \"Search\"\nstatus: approved\nsummary: \"Finds recipes.\"\n---\n";
    let import =
        "---\nid: fd-000004\ntitle: Import\nstatus: draft\nsummary: \"Imports recipes.\"\n---\n";
    let archive = "---\nid: fd-000005\ntitle: \"Old recipes\"\nstatus: superseded\nsummary: \"Retired.\"\nsuperseded_by: fd-000002\n---\n";
    let mut files = vec![
        ("manifestdocs.yaml", "name: cookbook\n"),
        ("recipes/recipes.md", recipes.as_str()),
        ("recipes/search/search.md", search),
        ("recipes/search/ranking-rules.md", "# Ranking\n"),
        (
            "recipes/search/decisions/2026-10-28-engine.md",
            "---\nid: dc-0000001\n---\n",
        ),
        ("recipes/import/import.md", import),
        ("recipes-archive/recipes-archive.md", archive),
        ("guides/kitchen/knives/knives.md", knives.as_str()),
        ("notes/notes.md", "# Notes\n"),
        ("vendor/manifestdocs.yaml", "name: vendor\n"),
        ("vendor/vendor.md", vendor.as_str()),
    ];
    if with_root {
        files.push(("cookbook.md", root.as_str()));
    }
    Fixture::new(name, &files)
}

#[test]
fn outlines_every_node_in_folder_order() {
    let fixture = cookbook("tree-outline", true);

    let run = fixture.run("", &["tree"]);

    assert_eq!(
        run.stdout,
        "Cookbook (fd-000001, draft)  cookbook.md\n\
         \x20 Knives (fd-000006, draft)  guides/kitchen/knives/knives.md\n\
         \x20 Recipes (fd-000002, draft)  recipes/recipes.md\n\
         \x20   import (fd-000004, has problems)  recipes/import/import.md\n\
         \x20   Search (fd-000003, approved)  recipes/search/search.md\n\
         \x20 Old recipes (fd-000005, superseded)  recipes-archive/recipes-archive.md\n"
    );
    assert_eq!(run.stderr, "");
    assert_eq!(run.code, 0);
}

#[test]
fn limits_the_outline_to_a_depth() {
    let fixture = cookbook("tree-depth", true);

    let one = fixture.run("", &["tree", "--depth", "1"]);
    let zero = fixture.run("", &["tree", "--depth", "0"]);

    assert_eq!(
        one.stdout,
        "Cookbook (fd-000001, draft)  cookbook.md\n\
         \x20 Knives (fd-000006, draft)  guides/kitchen/knives/knives.md\n\
         \x20 Recipes (fd-000002, draft)  recipes/recipes.md\n\
         \x20 Old recipes (fd-000005, superseded)  recipes-archive/recipes-archive.md\n"
    );
    assert_eq!(zero.stdout, "Cookbook (fd-000001, draft)  cookbook.md\n");
}

#[test]
fn starts_the_outline_at_a_node_or_folder() {
    let fixture = cookbook("tree-start", true);

    let node_folder = fixture.run("", &["tree", "recipes"]);
    let node_file = fixture.run("", &["tree", "recipes/search/search.md"]);
    let pass_through = fixture.run("", &["tree", "guides/"]);

    assert_eq!(
        node_folder.stdout,
        "Recipes (fd-000002, draft)  recipes/recipes.md\n\
         \x20 import (fd-000004, has problems)  recipes/import/import.md\n\
         \x20 Search (fd-000003, approved)  recipes/search/search.md\n"
    );
    assert_eq!(
        node_file.stdout,
        "Search (fd-000003, approved)  recipes/search/search.md\n"
    );
    assert_eq!(
        pass_through.stdout,
        "Knives (fd-000006, draft)  guides/kitchen/knives/knives.md\n"
    );
}

#[test]
fn names_paths_from_the_current_folder() {
    let fixture = cookbook("tree-subfolder", true);

    let whole = fixture.run("recipes", &["tree", "--depth", "1"]);
    let search = fixture.run("recipes", &["tree", "search"]);

    assert_eq!(
        whole.stdout,
        "Cookbook (fd-000001, draft)  ../cookbook.md\n\
         \x20 Knives (fd-000006, draft)  ../guides/kitchen/knives/knives.md\n\
         \x20 Recipes (fd-000002, draft)  recipes.md\n\
         \x20 Old recipes (fd-000005, superseded)  ../recipes-archive/recipes-archive.md\n"
    );
    assert_eq!(
        search.stdout,
        "Search (fd-000003, approved)  search/search.md\n"
    );
}

#[test]
fn outlines_top_level_nodes_when_there_is_no_root_node() {
    let fixture = cookbook("tree-no-root", false);

    let run = fixture.run("", &["tree", "--depth", "1"]);

    assert_eq!(
        run.stdout,
        "Knives (fd-000006, draft)  guides/kitchen/knives/knives.md\n\
         Recipes (fd-000002, draft)  recipes/recipes.md\n\
         Old recipes (fd-000005, superseded)  recipes-archive/recipes-archive.md\n"
    );
}

#[test]
fn stops_when_the_start_isnt_a_node_or_folder_in_the_tree() {
    let fixture = Fixture::new(
        "tree-bad-start",
        &[
            ("README.md", "# Readme\n"),
            ("docs/manifestdocs.yaml", "name: cookbook\n"),
            ("docs/cookbook.md", &node("fd-000001", "Cookbook")),
            ("docs/notes.md", "# Notes\n"),
        ],
    );
    let cases = [
        ("nowhere", "error: no file or folder `nowhere`\n"),
        (
            "notes.md",
            "error: `notes.md` isn't a node file or a folder\n",
        ),
        (
            "../README.md",
            "error: `../README.md` is outside the tree\n",
        ),
    ];
    for (start, expected) in cases {
        let run = fixture.run("docs", &["tree", start]);

        assert_eq!(run.stderr, expected, "{start}");
        assert_eq!(run.stdout, "", "{start}");
        assert_eq!(run.code, 2, "{start}");
    }
}

#[test]
fn keeps_nodes_with_broken_frontmatter_in_place() {
    // The root's title is unquoted, and the Recipes block never closes.
    let root = "---\nid: fd-000001\ntitle: Cookbook\nstatus: draft\nsummary: \"Recipes.\"\n---\n";
    let recipes = "---\nid: fd-000002\ntitle: \"Recipes\"\nstatus: draft\nsummary: \"All recipes.\"\n\n# Recipes\n";
    let search = node("fd-000003", "Search");
    let fixture = Fixture::new(
        "tree-broken",
        &[
            ("manifestdocs.yaml", "name: cookbook\n"),
            ("cookbook.md", root),
            ("recipes/recipes.md", recipes),
            ("recipes/notes.md", "# Notes\n"),
            ("recipes/search/search.md", &search),
        ],
    );

    let tree = fixture.run("", &["tree"]);
    let show = fixture.run("", &["show", "recipes"]);

    assert_eq!(
        tree.stdout,
        "cookbook (fd-000001, has problems)  cookbook.md\n\
         \x20 recipes (fd-000002, has problems)  recipes/recipes.md\n\
         \x20   Search (fd-000003, draft)  recipes/search/search.md\n"
    );
    assert_eq!(
        show.stdout,
        "recipes/recipes.md:1: error: frontmatter isn't valid YAML: no closing `---` line\n"
    );
    assert_eq!(show.code, 1);
}

#[test]
fn reads_a_tree_whose_supporting_doc_isnt_utf8() {
    let fixture = cookbook("tree-latin1", true);
    std::fs::write(fixture.path("recipes/search/latin1.md"), b"caf\xe9\n").unwrap();

    let tree = fixture.run("", &["tree", "recipes/search"]);
    let show = fixture.run("", &["show", "recipes/search"]);

    assert_eq!(
        tree.stdout,
        "Search (fd-000003, approved)  recipes/search/search.md\n"
    );
    assert!(
        show.stdout
            .ends_with("- recipes/search/latin1.md\n- recipes/search/ranking-rules.md\n"),
        "{}",
        show.stdout
    );
}
