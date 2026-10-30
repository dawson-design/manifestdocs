//! `context --json`.

use serde_json::{Value, json};

use super::{cookbook, parsed};
use crate::docs_source::{git, workspace};
use crate::support::{Fixture, GhIssue, Run, node};

/// The size the text output's last line states.
fn text_tokens(run: &Run) -> u64 {
    let line = run.stdout.lines().last().unwrap_or_default();
    let size = line.strip_prefix("Estimated size: ").unwrap_or_default();
    let digits: String = size.split(' ').next().unwrap_or_default().replace(',', "");
    digits
        .parse()
        .unwrap_or_else(|_| panic!("no estimate: {}", run.stdout))
}

#[test]
fn context_prints_the_pieces_in_the_order_text_prints_them() {
    let fixture = cookbook("json-context");

    let run = fixture.run("", &["context", "fd-7c2e41", "--json"]);
    let text = fixture.run("", &["context", "fd-7c2e41"]);

    assert_eq!(
        parsed(&run),
        json!({
            "schema": 1,
            "command": "context",
            "data": {
                "target": "fd-7c2e41",
                "version": null,
                "pieces": [
                    {
                        "kind": "target",
                        "heading": "Feature: Search (fd-7c2e41, approved)",
                        "path": "recipes/search/search.md",
                        "text": "# Search\n\nRequirements, with the [ranking rules](ranking-rules.md)."
                    },
                    {
                        "kind": "parent",
                        "heading": "Parent: Recipes (fd-0a0002, draft)",
                        "path": "recipes/recipes.md",
                        "text": "# Recipes"
                    },
                    {
                        "kind": "project",
                        "heading": "Project: Cookbook (fd-0a0001, draft)",
                        "path": "cookbook.md",
                        "text": "# Cookbook"
                    },
                    {
                        "kind": "linked_doc",
                        "heading": "recipes/search/ranking-rules.md",
                        "path": "recipes/search/ranking-rules.md",
                        "text": "# Ranking"
                    }
                ],
                "omitted": [],
                "tokens": text_tokens(&text),
                "budget": null
            }
        })
    );
    assert_eq!(run.code, 0);
}

#[test]
fn context_lists_what_the_budget_left_out() {
    let fixture = cookbook("json-context-budget");

    let run = fixture.run("", &["context", "fd-7c2e41", "--budget", "80", "--json"]);
    let text = fixture.run("", &["context", "fd-7c2e41", "--budget", "80"]);

    let data = parsed(&run)["data"].clone();
    assert_eq!(
        (&data["omitted"], &data["budget"]),
        (
            &json!([
                { "kind": "parent", "path": "recipes/recipes.md", "tokens": 22 },
                { "kind": "project", "path": "cookbook.md", "tokens": 18 },
                { "kind": "linked_doc", "path": "recipes/search/ranking-rules.md", "tokens": 16 }
            ]),
            &json!(80)
        )
    );
    assert!(text.stdout.contains(
        "Omitted for budget: recipes/recipes.md (22 tokens), cookbook.md (18 tokens), \
         recipes/search/ranking-rules.md (16 tokens)\n"
    ));
    assert_eq!(data["tokens"], json!(text_tokens(&text)));
    assert_eq!(data["pieces"].as_array().map(Vec::len), Some(1));
}

const GITHUB: &str =
    "name: cookbook\ngithub:\n  host: github.com\n  docs_repo: acme/cookbook-docs\n";

/// Issue #60, under #41, names the Search node in a `feature:` line.
fn issue_tree(name: &str) -> Fixture {
    let fixture = Fixture::new(
        name,
        &[
            ("manifestdocs.yaml", GITHUB),
            ("cookbook.md", &node("fd-0a0001", "Cookbook")),
            ("search/search.md", &node("fd-7c2e41", "Search")),
        ],
    );
    let parent = GhIssue {
        title: "Recipe search",
        sub_issues: &["acme/cookbook-docs#60"],
        ..GhIssue::default()
    };
    fixture.gh_issue_with("github.com", "acme/cookbook-docs#41", &parent);
    let issue = GhIssue {
        title: "Cooking time",
        body: "feature: fd-7c2e41\n",
        issue_type: Some("Task"),
        parent: Some("acme/cookbook-docs#41"),
        ..GhIssue::default()
    };
    fixture.gh_issue_with("github.com", "acme/cookbook-docs#60", &issue);
    fixture
}

#[test]
fn context_of_an_issue_marks_a_parent_it_cant_read_and_exits_1() {
    let fixture = issue_tree("json-context-issue-unreadable");
    fixture.gh_unreadable_issue("github.com", "acme/cookbook-docs#41");

    let run = fixture.run_with_gh("", &["context", "acme/cookbook-docs#60", "--json"]);

    assert_eq!(
        parsed(&run)["data"]["pieces"][1],
        json!({
            "kind": "parent_work",
            "heading": "Parent work",
            "path": null,
            "text": "- #41: can't read it on GitHub: Resource not accessible by integration"
        })
    );
    assert_eq!(run.code, 1);
}

#[test]
fn context_of_an_issue_prints_the_issue_and_its_parent_work_with_no_path() {
    let fixture = issue_tree("json-context-issue");

    let run = fixture.run_with_gh("", &["context", "acme/cookbook-docs#60", "--json"]);

    let data = parsed(&run)["data"].clone();
    let pieces = data["pieces"].as_array().expect("pieces is a list");
    assert_eq!(data["target"], json!("acme/cookbook-docs#60"));
    assert_eq!(
        Value::from(pieces[..3].to_vec()),
        json!([
            {
                "kind": "issue",
                "heading": "Issue: Cooking time (Task, open)",
                "path": null,
                "text": "feature: fd-7c2e41"
            },
            {
                "kind": "parent_work",
                "heading": "Parent work",
                "path": null,
                "text": "- #41 Recipe search (open, 0 of 1 done)"
            },
            {
                "kind": "target",
                "heading": "Feature: Search (fd-7c2e41, draft)",
                "path": "search/search.md",
                "text": "# Search"
            }
        ])
    );
    assert_eq!(run.code, 0, "{}", run.stderr);
}

#[test]
fn context_on_a_detached_head_has_no_ref_in_version() {
    let (fixture, _, feature) = workspace("json-context-detached");
    git(&fixture, "work/docs", &["checkout", "-q", "--detach"]);

    let run = fixture.run("work/docs", &["context", "fd-7c2e41", "--json"]);

    assert_eq!(
        parsed(&run)["data"]["version"],
        json!({
            "ref": null,
            "commit": feature,
            "date": "2026-10-27",
            "uncommitted": true,
            "default_branch": "main"
        })
    );
}

#[test]
fn context_names_the_branch_commit_and_changes_in_version() {
    let (fixture, _, feature) = workspace("json-context-version");

    let run = fixture.run("work/docs", &["context", "fd-7c2e41", "--json"]);

    assert_eq!(
        parsed(&run)["data"]["version"],
        json!({
            "ref": "feature/x",
            "commit": feature,
            "date": "2026-10-27",
            "uncommitted": true,
            "default_branch": "main"
        })
    );
}

/// From another repo, the context is `origin/HEAD`'s, so it has no uncommitted changes.
#[test]
fn context_out_writes_the_json_to_the_file() {
    let (fixture, main, _) = workspace("json-context-out");
    let out = ".context/context.json";

    let run = fixture.run(
        "work/web",
        &["context", "fd-7c2e41", "--json", "--out", out],
    );

    let written = fixture.read("work/web/.context/context.json");
    let written: Value = serde_json::from_str(&written).expect("the file is JSON");
    assert_eq!(
        written["data"]["version"],
        json!({
            "ref": "origin/main",
            "commit": main,
            "date": "2026-10-27",
            "uncommitted": false,
            "default_branch": null
        })
    );
    assert_eq!(run.stderr, format!("Wrote {out}\n"));
    assert_eq!((run.code, run.stdout.as_str()), (0, ""));
}

#[test]
fn context_of_a_broken_node_prints_its_errors_as_an_error_object_and_exits_1() {
    let fixture = cookbook("json-context-broken");

    let run = fixture.run("", &["context", "fd-0a0006", "--json"]);

    assert_eq!(
        parsed(&run),
        json!({
            "schema": 1,
            "command": "context",
            "error": "recipes/import/import.md:3: error: `title` must be double-quoted"
        })
    );
    assert_eq!(run.code, 1);
}

#[test]
fn context_with_json_and_a_format_prints_an_error_object() {
    let fixture = cookbook("json-context-format");

    let run = fixture.run("", &["context", "fd-7c2e41", "--format", "md", "--json"]);

    assert_eq!(
        parsed(&run),
        json!({
            "schema": 1,
            "command": "context",
            "error": "the argument '--format <FORMAT>' cannot be used with '--json'"
        })
    );
    assert_eq!((run.code, run.stderr.as_str()), (2, ""));
}
