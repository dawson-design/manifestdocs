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

/// A Search node with a chain of three page-size records, a record whose predecessor is
/// missing, a record that supersedes itself, and a record whose frontmatter is broken.
pub fn records(name: &str) -> Fixture {
    let root = node("fd-0a0001", "Cookbook");
    let search = node("fd-7c2e41", "Search");
    let newest = "---\nid: dc-3f1a8c2\nnode: fd-7c2e41\ntitle: \"Show 20 recipes per page\"\nstatus: accepted\ntopic: page-size\ndecided_by: octocat\ndate: 2026-10-28\nsource: [example-org/cookbook-web#212]\napplies_to: [example-org/cookbook-web:src/search/page.ts]\nsupersedes: dc-2b8e4c5\n---\n\n**Decision.** Show 20 recipes per page.\n\n**Why.** Readers rarely scroll past 20.\n";
    let middle = "---\nid: dc-2b8e4c5\nnode: fd-7c2e41\ntitle: \"Show 50 recipes per page\"\nstatus: superseded\ntopic: page-size\ndate: 2026-10-14\nsupersedes: dc-1a7f3b4\n---\n\n**Decision.** Show 50 recipes per page.\n";
    let oldest = "---\nid: dc-1a7f3b4\nnode: fd-7c2e41\ntitle: \"Show 10 recipes per page\"\nstatus: superseded\ntopic: page-size\ndate: 2026-10-01\n---\n\n**Decision.** Show 10 recipes per page.\n";
    let orphan = "---\nid: dc-3f1a999\nnode: fd-7c2e41\ntitle: \"Rank by rating\"\nstatus: proposed\ntopic: ranking\ndate: 2026-10-29\nsupersedes: dc-00000ff\n---\n";
    let broken = "---\nid: dc-4c4c4c4\nnode: fd-7c2e41\ntitle: Broken\nstatus: final\ntopic: broken\ndate: 2026-10-30\n---\n";
    let looped = "---\nid: dc-5e1f5e1\nnode: fd-7c2e41\ntitle: \"Cache results\"\nstatus: accepted\ntopic: cache\ndate: 2026-10-31\nsupersedes: dc-5e1f5e1\n---\n\n**Decision.** Cache results.\n";
    Fixture::new(
        name,
        &[
            ("manifestdocs.yaml", "name: cookbook\n"),
            ("cookbook.md", &root),
            ("search/search.md", &search),
            ("search/decisions/2026-10-28-page-size.md", newest),
            ("search/decisions/2026-10-14-page-size.md", middle),
            ("search/decisions/2026-10-01-page-size.md", oldest),
            ("search/decisions/2026-10-29-ranking.md", orphan),
            ("search/decisions/2026-10-30-broken.md", broken),
            ("search/decisions/2026-10-31-cache.md", looped),
        ],
    )
}

#[test]
fn shows_a_record_that_supersedes_itself_once() {
    let fixture = records("show-record-self");

    let run = fixture.run("", &["show", "dc-5e1f"]);

    assert_eq!(
        run.stdout,
        "Cache results (dc-5e1f5e1, accepted)\n\
         File: search/decisions/2026-10-31-cache.md\n\
         Node: Search (fd-7c2e41)\n\
         Topic: cache\n\
         Date: 2026-10-31\n\
         Supersedes: Cache results (dc-5e1f5e1)\n\
         \n\
         **Decision.** Cache results.\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn shows_a_record_by_id_or_prefix_with_the_chain_it_supersedes() {
    let fixture = records("show-record-chain");
    let expected = "Show 20 recipes per page (dc-3f1a8c2, accepted)\n\
                    File: search/decisions/2026-10-28-page-size.md\n\
                    Node: Search (fd-7c2e41)\n\
                    Topic: page-size\n\
                    Date: 2026-10-28\n\
                    Decided by: octocat\n\
                    Source: example-org/cookbook-web#212\n\
                    Applies to: example-org/cookbook-web:src/search/page.ts\n\
                    Supersedes: Show 50 recipes per page (dc-2b8e4c5)\n\
                    \n\
                    **Decision.** Show 20 recipes per page.\n\
                    \n\
                    **Why.** Readers rarely scroll past 20.\n\
                    \n\
                    Show 50 recipes per page (dc-2b8e4c5, superseded)\n\
                    File: search/decisions/2026-10-14-page-size.md\n\
                    Node: Search (fd-7c2e41)\n\
                    Topic: page-size\n\
                    Date: 2026-10-14\n\
                    Supersedes: Show 10 recipes per page (dc-1a7f3b4)\n\
                    \n\
                    **Decision.** Show 50 recipes per page.\n\
                    \n\
                    Show 10 recipes per page (dc-1a7f3b4, superseded)\n\
                    File: search/decisions/2026-10-01-page-size.md\n\
                    Node: Search (fd-7c2e41)\n\
                    Topic: page-size\n\
                    Date: 2026-10-01\n\
                    \n\
                    **Decision.** Show 10 recipes per page.\n";

    for id in ["dc-3f1a8c2", "dc-3f1a8"] {
        let run = fixture.run("", &["show", id]);

        assert_eq!(run.stdout, expected, "{id}");
        assert_eq!(run.code, 0, "{id}");
    }
}

#[test]
fn shows_a_record_whose_predecessor_is_missing_by_id_only() {
    let fixture = records("show-record-orphan");

    let run = fixture.run("search", &["show", "dc-3f1a9"]);

    assert_eq!(
        run.stdout,
        "Rank by rating (dc-3f1a999, proposed)\n\
         File: decisions/2026-10-29-ranking.md\n\
         Node: Search (fd-7c2e41)\n\
         Topic: ranking\n\
         Date: 2026-10-29\n\
         Supersedes: dc-00000ff\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn prints_the_errors_of_a_record_whose_frontmatter_is_broken() {
    let fixture = records("show-record-broken");

    let run = fixture.run("", &["show", "dc-4c4c"]);

    assert_eq!(
        run.stdout,
        "search/decisions/2026-10-30-broken.md:4: error: `title` must be double-quoted\n\
         search/decisions/2026-10-30-broken.md:5: error: status `final` isn't a record status: expected `proposed`, `accepted`, `superseded` or `rejected`\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn stops_when_the_argument_names_no_single_record() {
    let fixture = records("show-record-none");
    let cases = [
        (
            "dc-3f1a",
            "error: `dc-3f1a` matches 2 decision records:\n  dc-3f1a8c2  search/decisions/2026-10-28-page-size.md\n  dc-3f1a999  search/decisions/2026-10-29-ranking.md\n",
        ),
        ("dc-ffff", "error: no decision record `dc-ffff`\n"),
        (
            "dc-3f1",
            "error: `dc-3f1` isn't a decision ID: expected `dc-` and 4 to 7 lowercase hex characters\n",
        ),
    ];
    for (id, expected) in cases {
        let run = fixture.run("", &["show", id]);

        assert_eq!(run.stderr, expected, "{id}");
        assert_eq!(run.stdout, "", "{id}");
        assert_eq!(run.code, 2, "{id}");
    }
}
