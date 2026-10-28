//! `manifestdocs show`.

use crate::support::{Fixture, node};

/// A cookbook tree whose Search node has every optional field, supporting docs and a child.
fn cookbook(name: &str) -> Fixture {
    let root = node("fd-0a0001", "Cookbook");
    let recipes = node("fd-0a0002", "Recipes");
    let filters = node("fd-7c2e99", "Filters");
    let search = "---\nid: fd-7c2e41\ntitle: \"Search\"\nstatus: approved\nsummary: \"Finds recipes by ingredient.\"\nrepos: [example-org/cookbook-web]\nwork: [example-org/cookbook-docs#41, example-org/cookbook-docs#58]\n---\n\n# Search\n\nRequirements, with the [ranking rules](ranking-rules.md).\n\n";
    let old = "---\nid: fd-0a0005\ntitle: \"Old search\"\nstatus: superseded\nsummary: \"Retired.\"\nsuperseded_by: fd-7c2e41\n---\n";
    let legacy = "---\nid: fd-0a0007\ntitle: \"Legacy\"\nstatus: superseded\nsummary: \"Gone.\"\nsuperseded_by: fd-ffffff\n---\n";
    let import = "---\nid: fd-0a0006\ntitle: Import\nstatus: live\nsummary: \"Imports recipes.\"\nowner: sam\n---\n";
    Fixture::new(
        name,
        &[
            ("manifestdocs.yaml", "name: cookbook\n"),
            ("cookbook.md", &root),
            ("glossary.md", "# Glossary\n"),
            ("recipes/recipes.md", &recipes),
            ("recipes/search/search.md", search),
            ("recipes/search/ranking-rules.md", "# Ranking\n"),
            ("recipes/search/images/flow.png", "png"),
            (
                "recipes/search/decisions/2026-10-28-engine.md",
                "---\nid: dc-0000001\n---\n",
            ),
            ("recipes/search/filters/filters.md", &filters),
            ("recipes/search/filters/notes.md", "# Notes\n"),
            ("recipes/old-search/old-search.md", old),
            ("recipes/import/import.md", import),
            ("recipes/legacy/legacy.md", legacy),
        ],
    )
}

const SEARCH: &str = "Search (fd-7c2e41, approved)\n\
                      File: recipes/search/search.md\n\
                      Parent: Recipes (fd-0a0002)\n\
                      Summary: Finds recipes by ingredient.\n\
                      Repos: example-org/cookbook-web\n\
                      Work: example-org/cookbook-docs#41, example-org/cookbook-docs#58\n\
                      \n\
                      # Search\n\
                      \n\
                      Requirements, with the [ranking rules](ranking-rules.md).\n\
                      \n\
                      Supporting docs:\n\
                      - recipes/search/images/flow.png\n\
                      - recipes/search/ranking-rules.md\n";

#[test]
fn shows_a_node_by_id_prefix_folder_or_file() {
    let fixture = cookbook("show-select");

    for node in [
        "fd-7c2e41",
        "fd-7c2e4",
        "recipes/search",
        "recipes/search/",
        "recipes/search/search.md",
    ] {
        let run = fixture.run("", &["show", node]);

        assert_eq!(run.stdout, SEARCH, "{node}");
        assert_eq!(run.code, 0, "{node}");
    }
}

#[test]
fn names_paths_from_the_current_folder() {
    let fixture = cookbook("show-subfolder");

    let run = fixture.run("recipes", &["show", "search"]);

    assert!(
        run.stdout.starts_with(
            "Search (fd-7c2e41, approved)\nFile: search/search.md\nParent: Recipes (fd-0a0002)\n"
        ),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout
            .ends_with("Supporting docs:\n- search/images/flow.png\n- search/ranking-rules.md\n"),
        "{}",
        run.stdout
    );
}

#[test]
fn shows_the_root_node_and_a_superseded_node() {
    let fixture = cookbook("show-root-superseded");

    let root = fixture.run("", &["show", "."]);
    let old = fixture.run("", &["show", "recipes/old-search"]);
    let legacy = fixture.run("", &["show", "recipes/legacy"]);

    assert_eq!(
        root.stdout,
        "Cookbook (fd-0a0001, draft)\nFile: cookbook.md\nSummary: Cookbook summary.\n\n\
         # Cookbook\n\nSupporting docs:\n- glossary.md\n"
    );
    assert_eq!(
        old.stdout,
        "Old search (fd-0a0005, superseded)\nFile: recipes/old-search/old-search.md\n\
         Parent: Recipes (fd-0a0002)\nSummary: Retired.\nSuperseded by: Search (fd-7c2e41)\n"
    );
    assert!(
        legacy
            .stdout
            .ends_with("Summary: Gone.\nSuperseded by: fd-ffffff\n"),
        "{}",
        legacy.stdout
    );
}

#[test]
fn prints_the_errors_of_a_node_whose_frontmatter_is_broken() {
    let fixture = cookbook("show-broken");

    let run = fixture.run("", &["show", "fd-0a0006"]);

    assert_eq!(
        run.stdout,
        "recipes/import/import.md:3: error: `title` must be double-quoted\n\
         recipes/import/import.md:4: error: status `live` isn't in `statuses` in manifestdocs.yaml\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn stops_when_the_argument_names_no_single_node() {
    let fixture = cookbook("show-no-node");
    let cases = [
        (
            "fd-7c2e",
            "error: `fd-7c2e` matches 2 nodes:\n  fd-7c2e41  recipes/search/search.md\n  fd-7c2e99  recipes/search/filters/filters.md\n",
        ),
        ("fd-ffff", "error: no node `fd-ffff`\n"),
        (
            "fd-7c2",
            "error: `fd-7c2` isn't a node ID: expected `fd-` and 4 to 6 lowercase hex characters\n",
        ),
        (
            "recipes/search/ranking-rules.md",
            "error: `recipes/search/ranking-rules.md` isn't a node file or node folder\n",
        ),
        ("recipes/gone", "error: no file or folder `recipes/gone`\n"),
    ];
    for (node, expected) in cases {
        let run = fixture.run("", &["show", node]);

        assert_eq!(run.stderr, expected, "{node}");
        assert_eq!(run.stdout, "", "{node}");
        assert_eq!(run.code, 2, "{node}");
    }
}
