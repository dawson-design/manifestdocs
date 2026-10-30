//! `--json` output, schema version 1.

mod context;

use serde_json::{Value, json};

use crate::support::{Fixture, Run, node};

fn parsed(run: &Run) -> Value {
    serde_json::from_str(&run.stdout).expect("stdout is one JSON object")
}

/// A tree with no root node and a broken link, so `check` finds a warning with no line and an
/// error with one.
fn broken(name: &str) -> Fixture {
    let recipes = node("fd-000002", "Recipes") + "\nSee [ranking](ranking.md).\n";
    Fixture::new(
        name,
        &[
            ("manifestdocs.yaml", "name: demo\n"),
            ("recipes/recipes.md", &recipes),
        ],
    )
}

/// A cookbook tree with a superseded node, a node with frontmatter errors and a node with every
/// optional field and supporting docs.
fn cookbook(name: &str) -> Fixture {
    let root = node("fd-0a0001", "Cookbook");
    let recipes = node("fd-0a0002", "Recipes");
    let search = "---\nid: fd-7c2e41\ntitle: \"Search\"\nstatus: approved\nsummary: \"Finds recipes by ingredient.\"\nrepos: [example-org/cookbook-web]\nwork: [example-org/cookbook-docs#41]\n---\n\n# Search\n\nRequirements, with the [ranking rules](ranking-rules.md).\n\n";
    let old = "---\nid: fd-0a0005\ntitle: \"Old search\"\nstatus: superseded\nsummary: \"Retired.\"\nsuperseded_by: fd-7c2e41\n---\n";
    let import =
        "---\nid: fd-0a0006\ntitle: Import\nstatus: draft\nsummary: \"Imports recipes.\"\n---\n";
    Fixture::new(
        name,
        &[
            ("manifestdocs.yaml", "name: cookbook\n"),
            ("cookbook.md", &root),
            ("recipes/recipes.md", &recipes),
            ("recipes/search/search.md", search),
            ("recipes/search/ranking-rules.md", "# Ranking\n"),
            ("recipes/search/images/flow.png", "png"),
            ("recipes/old-search/old-search.md", old),
            ("recipes/import/import.md", import),
        ],
    )
}

#[test]
fn check_prints_the_node_count_and_findings() {
    let fixture = broken("json-check-findings");

    let run = fixture.run("", &["check", "--json"]);

    assert_eq!(
        parsed(&run),
        json!({
            "schema": 1,
            "command": "check",
            "data": {
                "nodes": 1,
                "findings": [
                    {
                        "file": "manifestdocs.yaml",
                        "line": null,
                        "severity": "warning",
                        "code": "no_root_node",
                        "message": "no root node `demo.md`"
                    },
                    {
                        "file": "recipes/recipes.md",
                        "line": 10,
                        "severity": "error",
                        "code": "broken_link",
                        "message": "broken link `ranking.md`"
                    }
                ]
            }
        })
    );
    assert_eq!(run.code, 1);
}

#[test]
fn check_with_no_problems_prints_an_empty_list() {
    let root = node("fd-000001", "Demo");
    let fixture = Fixture::new(
        "json-check-clean",
        &[("manifestdocs.yaml", "name: demo\n"), ("demo.md", &root)],
    );

    let run = fixture.run("", &["check", "--json"]);

    assert_eq!(
        parsed(&run),
        json!({ "schema": 1, "command": "check", "data": { "nodes": 1, "findings": [] } })
    );
    assert_eq!(run.code, 0);
}

#[test]
fn check_from_a_subfolder_prints_paths_relative_to_it() {
    let fixture = broken("json-check-subfolder");

    let run = fixture.run("recipes", &["check", "--json"]);

    let output = parsed(&run);
    let findings = output["data"]["findings"].as_array();
    let files: Vec<&Value> = findings
        .expect("findings is a list")
        .iter()
        .map(|finding| &finding["file"])
        .collect();
    assert_eq!(
        files,
        [&json!("../manifestdocs.yaml"), &json!("recipes.md")]
    );
}

#[test]
fn tree_lists_nodes_in_outline_order_with_parent_ids() {
    let fixture = cookbook("json-tree");

    let run = fixture.run("", &["tree", "--json"]);

    assert_eq!(
        parsed(&run),
        json!({
            "schema": 1,
            "command": "tree",
            "data": {
                "nodes": [
                    { "id": "fd-0a0001", "title": "Cookbook", "status": "draft", "file": "cookbook.md", "parent": null },
                    { "id": "fd-0a0002", "title": "Recipes", "status": "draft", "file": "recipes/recipes.md", "parent": "fd-0a0001" },
                    { "id": "fd-0a0006", "title": null, "status": null, "file": "recipes/import/import.md", "parent": "fd-0a0002" },
                    { "id": "fd-0a0005", "title": "Old search", "status": "superseded", "file": "recipes/old-search/old-search.md", "parent": "fd-0a0002" },
                    { "id": "fd-7c2e41", "title": "Search", "status": "approved", "file": "recipes/search/search.md", "parent": "fd-0a0002" }
                ]
            }
        })
    );
    assert_eq!(run.code, 0);
}

#[test]
fn tree_of_a_folder_names_the_parent_above_it() {
    let fixture = cookbook("json-tree-folder");

    let run = fixture.run("", &["tree", "recipes/search", "--json"]);

    assert_eq!(
        parsed(&run)["data"],
        json!({
            "nodes": [
                { "id": "fd-7c2e41", "title": "Search", "status": "approved", "file": "recipes/search/search.md", "parent": "fd-0a0002" }
            ]
        })
    );
}

#[test]
fn tree_from_a_subfolder_prints_paths_relative_to_it() {
    let fixture = cookbook("json-tree-subfolder");

    let run = fixture.run("recipes", &["tree", "--depth", "1", "--json"]);

    assert_eq!(
        parsed(&run)["data"],
        json!({
            "nodes": [
                { "id": "fd-0a0001", "title": "Cookbook", "status": "draft", "file": "../cookbook.md", "parent": null },
                { "id": "fd-0a0002", "title": "Recipes", "status": "draft", "file": "recipes.md", "parent": "fd-0a0001" }
            ]
        })
    );
}

#[test]
fn show_prints_a_node_in_full() {
    let fixture = cookbook("json-show");

    let run = fixture.run("", &["show", "fd-7c2e", "--json"]);

    assert_eq!(
        parsed(&run),
        json!({
            "schema": 1,
            "command": "show",
            "data": {
                "id": "fd-7c2e41",
                "title": "Search",
                "status": "approved",
                "summary": "Finds recipes by ingredient.",
                "file": "recipes/search/search.md",
                "parent": "fd-0a0002",
                "repos": ["example-org/cookbook-web"],
                "work": ["example-org/cookbook-docs#41"],
                "superseded_by": null,
                "body": "# Search\n\nRequirements, with the [ranking rules](ranking-rules.md).",
                "supporting_docs": ["recipes/search/images/flow.png", "recipes/search/ranking-rules.md"],
                "findings": []
            }
        })
    );
    assert_eq!(run.code, 0);
}

#[test]
fn show_prints_null_and_empty_lists_for_missing_values() {
    let fixture = cookbook("json-show-root");

    let run = fixture.run("", &["show", "cookbook.md", "--json"]);

    assert_eq!(
        parsed(&run)["data"],
        json!({
            "id": "fd-0a0001",
            "title": "Cookbook",
            "status": "draft",
            "summary": "Cookbook summary.",
            "file": "cookbook.md",
            "parent": null,
            "repos": [],
            "work": [],
            "superseded_by": null,
            "body": "# Cookbook",
            "supporting_docs": [],
            "findings": []
        })
    );
}

#[test]
fn show_names_a_superseded_nodes_replacement_by_full_id() {
    let fixture = cookbook("json-show-superseded");

    let run = fixture.run("", &["show", "recipes/old-search", "--json"]);

    let data = parsed(&run)["data"].clone();
    assert_eq!(
        (&data["status"], &data["superseded_by"]),
        (&json!("superseded"), &json!("fd-7c2e41"))
    );
}

#[test]
fn show_prints_a_broken_nodes_findings_and_exits_1() {
    let fixture = cookbook("json-show-broken");

    let run = fixture.run("", &["show", "fd-0a0006", "--json"]);

    assert_eq!(
        parsed(&run)["data"],
        json!({
            "id": "fd-0a0006",
            "title": null,
            "status": null,
            "summary": null,
            "file": "recipes/import/import.md",
            "parent": "fd-0a0002",
            "repos": null,
            "work": null,
            "superseded_by": null,
            "body": null,
            "supporting_docs": [],
            "findings": [
                {
                    "file": "recipes/import/import.md",
                    "line": 3,
                    "severity": "error",
                    "code": "unquoted_field",
                    "message": "`title` must be double-quoted"
                }
            ]
        })
    );
    assert_eq!(run.code, 1);
}

#[test]
fn show_from_a_subfolder_prints_paths_relative_to_it() {
    let fixture = cookbook("json-show-subfolder");

    let run = fixture.run("recipes/search", &["show", "fd-7c2e41", "--json"]);

    let data = parsed(&run)["data"].clone();
    assert_eq!(
        (&data["file"], &data["supporting_docs"]),
        (
            &json!("search.md"),
            &json!(["images/flow.png", "ranking-rules.md"])
        )
    );
}

#[test]
fn a_command_that_cannot_run_prints_an_error_object_and_exits_2() {
    let fixture = Fixture::new("json-no-tree", &[("notes.md", "# Notes\n")]);

    let run = fixture.run("", &["check", "--json"]);

    assert_eq!(
        parsed(&run),
        json!({
            "schema": 1,
            "command": "check",
            "error": "no manifestdocs.yaml in this folder or any folder above it, up to the repo root, and no .manifestdocs/config.yaml at or above it"
        })
    );
    assert_eq!((run.code, run.stderr.as_str()), (2, ""));
}

#[test]
fn an_argument_that_names_no_node_prints_an_error_object() {
    let fixture = cookbook("json-show-no-node");

    let run = fixture.run("", &["show", "fd-ffffff", "--json"]);

    assert_eq!(
        parsed(&run),
        json!({ "schema": 1, "command": "show", "error": "no node `fd-ffffff`" })
    );
    assert_eq!(run.code, 2);
}

/// A record's JSON shape isn't settled, so `--json` refuses instead of printing text.
#[test]
fn show_of_a_decision_record_prints_an_error_object() {
    let fixture = cookbook("json-show-record");

    let run = fixture.run("", &["show", "dc-0000001", "--json"]);

    assert_eq!(
        parsed(&run),
        json!({
            "schema": 1,
            "command": "show",
            "error": "`show --json` doesn't support decision records yet"
        })
    );
    assert_eq!(run.code, 2);
}

#[test]
fn an_invalid_option_value_prints_an_error_object() {
    let fixture = cookbook("json-bad-option");

    let run = fixture.run("", &["tree", "--depth", "x", "--json"]);

    assert_eq!(
        parsed(&run),
        json!({
            "schema": 1,
            "command": "tree",
            "error": "invalid value 'x' for '--depth <DEPTH>': invalid digit found in string"
        })
    );
    assert_eq!((run.code, run.stderr.as_str()), (2, ""));
}

#[test]
fn an_ambiguous_id_prefix_prints_the_whole_multi_line_error() {
    let fixture = Fixture::new(
        "json-ambiguous",
        &[
            ("manifestdocs.yaml", "name: demo\n"),
            ("demo.md", &node("fd-000001", "Demo")),
            ("a/a.md", &node("fd-7c2e41", "A")),
            ("b/b.md", &node("fd-7c2e99", "B")),
        ],
    );

    let run = fixture.run("", &["show", "fd-7c2e", "--json"]);

    assert_eq!(
        parsed(&run),
        json!({
            "schema": 1,
            "command": "show",
            "error": "`fd-7c2e` matches 2 nodes:\n  fd-7c2e41  a/a.md\n  fd-7c2e99  b/b.md"
        })
    );
    assert_eq!(run.code, 2);
}

#[test]
fn check_strict_exits_1_on_warnings_alone() {
    let fixture = Fixture::new(
        "json-strict",
        &[
            ("manifestdocs.yaml", "name: demo\n"),
            ("recipes/recipes.md", &node("fd-000002", "Recipes")),
        ],
    );

    let run = fixture.run("", &["check", "--strict", "--json"]);

    assert_eq!(
        parsed(&run)["data"],
        json!({
            "nodes": 1,
            "findings": [
                {
                    "file": "manifestdocs.yaml",
                    "line": null,
                    "severity": "warning",
                    "code": "no_root_node",
                    "message": "no root node `demo.md`"
                }
            ]
        })
    );
    assert_eq!(run.code, 1);
}

#[test]
fn check_names_a_skipped_nested_tree_as_a_folder() {
    let fixture = Fixture::new(
        "json-nested",
        &[
            ("manifestdocs.yaml", "name: demo\n"),
            ("demo.md", &node("fd-000001", "Demo")),
            ("vendor/manifestdocs.yaml", "name: vendor\n"),
        ],
    );

    let run = fixture.run("", &["check", "--json"]);

    assert_eq!(
        parsed(&run)["data"]["findings"],
        json!([
            {
                "file": "vendor/",
                "line": null,
                "severity": "warning",
                "code": "nested_tree_skipped",
                "message": "skipped nested tree"
            }
        ])
    );
    assert_eq!(run.code, 0);
}

#[test]
fn show_reads_back_a_body_with_quotes_and_backslashes_exactly() {
    let body = "# Paths\n\nSay \"hi\", then open `C:\\recipes\\new` and match `\\d+`.";
    let quoted = node("fd-000002", "Paths").replace("# Paths\n", body);
    let fixture = Fixture::new(
        "json-escapes",
        &[
            ("manifestdocs.yaml", "name: demo\n"),
            ("demo.md", &node("fd-000001", "Demo")),
            ("paths/paths.md", &quoted),
        ],
    );

    let run = fixture.run("", &["show", "fd-000002", "--json"]);

    assert_eq!(parsed(&run)["data"]["body"], json!(body));
}

#[test]
fn show_prints_an_empty_body_as_an_empty_string() {
    let fixture = cookbook("json-show-empty-body");

    let run = fixture.run("", &["show", "fd-0a0005", "--json"]);

    assert_eq!(parsed(&run)["data"]["body"], json!(""));
}

#[test]
fn help_with_json_prints_help_as_text() {
    let fixture = cookbook("json-help");

    let run = fixture.run("", &["show", "--json", "--help"]);

    assert!(serde_json::from_str::<Value>(&run.stdout).is_err());
    assert!(run.stdout.contains("Usage: manifestdocs show"));
    assert_eq!(run.code, 0);
}

#[test]
fn a_usage_error_without_json_stays_on_stderr() {
    let fixture = cookbook("json-usage-text");

    let run = fixture.run("", &["tree", "--depth", "x"]);

    assert_eq!(run.stdout, "");
    assert!(
        run.stderr
            .starts_with("error: invalid value 'x' for '--depth <DEPTH>'")
    );
    assert_eq!(run.code, 2);
}

#[test]
fn json_after_a_double_dash_is_not_a_request_for_json() {
    let fixture = cookbook("json-after-dashes");

    let run = fixture.run("", &["tree", "--depth", "x", "--", "--json"]);

    assert_eq!(run.stdout, "");
    assert!(
        run.stderr
            .starts_with("error: invalid value 'x' for '--depth <DEPTH>'")
    );
    assert_eq!(run.code, 2);
}
