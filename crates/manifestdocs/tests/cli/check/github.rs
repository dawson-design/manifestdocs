use serde_json::{Value, json};

use crate::support::{Fixture, node};

const GITHUB: &str = "name: demo\ngithub:\n  host: github.com\n  docs_repo: acme/cookbook-docs\n";

/// `folder/folder.md`, a node that lists `work` on line 6.
fn feature(id: &str, work: &str) -> String {
    format!(
        "---\nid: {id}\ntitle: \"Feature\"\nstatus: draft\nsummary: \"Feature summary.\"\nwork: [{work}]\n---\n"
    )
}

/// A tree with a root node and `feature/feature.md`, and no issues in a search of
/// `acme/cookbook-docs`.
fn tree(name: &str, config: &str, work: &str) -> Fixture {
    let root = node("fd-000001", "Demo");
    let fixture = Fixture::new(
        name,
        &[
            ("manifestdocs.yaml", config),
            ("demo.md", &root),
            ("feature/feature.md", &feature("fd-000002", work)),
        ],
    );
    fixture.gh_search("github.com", "acme/cookbook-docs", &[]);
    fixture
}

#[test]
fn passes_when_every_work_item_is_an_issue() {
    let fixture = tree("gh-found", GITHUB, "acme/cookbook-docs#41");
    fixture.gh_issue("github.com", "acme/cookbook-docs#41");

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(run.stderr, "");
    assert_eq!(run.code, 0);
}

#[test]
fn reports_a_work_item_that_isnt_an_issue() {
    let work = "acme/cookbook-docs#41, acme/cookbook-docs#99";
    let fixture = tree("gh-not-found", GITHUB, work);
    fixture.gh_issue("github.com", "acme/cookbook-docs#41");
    fixture.gh_no_issue("github.com", "acme/cookbook-docs#99");

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(
        run.stdout,
        "feature/feature.md:6: error: `work` names `acme/cookbook-docs#99`, which isn't an issue you can read on GitHub\n\n2 nodes: 1 error\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn reports_every_work_item_in_a_repo_that_doesnt_exist() {
    let fixture = tree("gh-no-repo", GITHUB, "acme/gone#1, acme/gone#2");
    fixture.gh_no_repo("github.com", "acme/gone");

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(
        run.stdout,
        "feature/feature.md:6: error: `work` names `acme/gone#1`, which isn't an issue you can read on GitHub\n\
         feature/feature.md:6: error: `work` names `acme/gone#2`, which isn't an issue you can read on GitHub\n\n\
         2 nodes: 2 errors\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn asks_about_an_issue_two_nodes_list_once() {
    let root = node("fd-000001", "Demo");
    let fixture = Fixture::new(
        "gh-shared-issue",
        &[
            ("manifestdocs.yaml", GITHUB),
            ("demo.md", &root),
            ("one/one.md", &feature("fd-000002", "acme/cookbook-docs#99")),
            ("two/two.md", &feature("fd-000003", "acme/cookbook-docs#99")),
        ],
    );
    fixture.gh_search("github.com", "acme/cookbook-docs", &[]);
    fixture.gh_no_issue("github.com", "acme/cookbook-docs#99");

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(
        run.stdout,
        "one/one.md:6: error: `work` names `acme/cookbook-docs#99`, which isn't an issue you can read on GitHub\n\
         two/two.md:6: error: `work` names `acme/cookbook-docs#99`, which isn't an issue you can read on GitHub\n\n\
         3 nodes: 2 errors\n"
    );
    let lookups: Vec<String> = fixture
        .gh_calls()
        .into_iter()
        .filter(|call| call.contains("repository("))
        .collect();
    assert_eq!(lookups.len(), 1);
    assert_eq!(lookups[0].matches("i99: issue(number: 99)").count(), 1);
}

#[test]
fn reads_a_repos_work_items_and_searches_with_read_only_queries() {
    let work = "acme/cookbook-docs#41, acme/cookbook-docs#7";
    let fixture = tree("gh-arguments", GITHUB, work);
    fixture.gh_issue("github.com", "acme/cookbook-docs#7");
    fixture.gh_issue("github.com", "acme/cookbook-docs#41");

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(run.code, 0);
    assert_eq!(
        fixture.gh_calls(),
        [
            "api graphql --hostname github.com \
             -f query=query($owner: String!, $name: String!) { repository(owner: $owner, name: $name) { \
             i7: issue(number: 7) { number } i41: issue(number: 41) { number } } } \
             -f owner=acme -f name=cookbook-docs",
            "api graphql --hostname github.com \
             -f query=query($q: String!, $cursor: String) { search(query: $q, type: ISSUE, first: 100, after: $cursor) { \
             issueCount pageInfo { hasNextPage endCursor } nodes { ... on Issue { number body } } } } \
             -f q=repo:acme/cookbook-docs is:issue in:body \"feature: fd-\" sort:created-asc",
        ]
    );
}

#[test]
fn splits_a_repo_with_many_work_items_into_batches() {
    let issues: Vec<String> = (1..=150)
        .map(|n| format!("acme/cookbook-docs#{n}"))
        .collect();
    let fixture = tree("gh-batches", GITHUB, &issues.join(", "));
    for issue in &issues {
        fixture.gh_issue("github.com", issue);
    }

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(run.stderr, "");
    let calls = fixture.gh_calls();
    let lookups: Vec<&String> = calls.iter().filter(|c| c.contains("repository(")).collect();
    assert_eq!(lookups.len(), 2);
    assert!(lookups[0].contains("i100: issue") && !lookups[0].contains("i101: issue"));
    assert!(lookups[1].contains("i101: issue") && lookups[1].contains("i150: issue"));
}

#[test]
fn reads_issues_from_the_configured_host() {
    let config = "name: demo\ngithub:\n  host: ghe.example.com\n  docs_repo: acme/cookbook-docs\n";
    let fixture = tree("gh-host", config, "acme/cookbook-docs#41");
    fixture.gh_issue("ghe.example.com", "acme/cookbook-docs#41");
    fixture.gh_search("ghe.example.com", "acme/cookbook-docs", &[]);

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(run.stderr, "");
    assert_eq!(run.code, 0);
}

#[test]
fn reports_an_issue_number_too_big_for_github_as_malformed() {
    let fixture = tree("gh-big-number", GITHUB, "acme/cookbook-docs#3000000000");

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(
        run.stdout,
        "feature/feature.md:6: error: malformed work item `acme/cookbook-docs#3000000000`: expected `owner/name#number`\n\n2 nodes: 1 error\n"
    );
    assert_eq!(run.stderr, "");
    let calls = fixture.gh_calls();
    assert!(
        !calls.iter().any(|call| call.contains("repository(")),
        "{calls:?}"
    );
}

#[test]
fn reads_an_owner_with_an_underscore() {
    let fixture = tree("gh-underscore", GITHUB, "alice_acme/notes#3");
    fixture.gh_issue("github.com", "alice_acme/notes#3");

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(run.stderr, "");
    assert_eq!(run.code, 0);
}

#[test]
fn skips_the_github_rules_when_gh_is_logged_out() {
    let fixture = tree("gh-logged-out", GITHUB, "acme/cookbook-docs#41");
    fixture.gh_logged_out();

    let run = fixture.run_with_gh("", &["check", "--strict"]);

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(
        run.stderr,
        "note: skipped the GitHub checks: `gh` isn't logged in to github.com; run `gh auth login --hostname github.com`\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn skips_the_github_rules_when_github_refuses_the_token() {
    let fixture = tree("gh-401", GITHUB, "acme/cookbook-docs#41");
    fixture.gh_unauthorized();

    let run = fixture.run_with_gh("", &["check", "--strict"]);

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(
        run.stderr,
        "note: skipped the GitHub checks: `gh` isn't logged in to github.com; run `gh auth login --hostname github.com`\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn skips_the_github_rules_when_gh_is_missing() {
    let fixture = tree("gh-missing", GITHUB, "acme/cookbook-docs#41");

    let run = fixture.run_without_gh("", &["check", "--strict"]);

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(
        run.stderr,
        "note: skipped the GitHub checks: `gh` isn't installed; install the GitHub CLI from https://cli.github.com\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn skips_only_the_repo_gh_fails_for() {
    let fixture = tree("gh-fails", GITHUB, "acme/cookbook-docs#99, acme/other#1");
    fixture.gh_no_issue("github.com", "acme/cookbook-docs#99");

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(
        run.stdout,
        "feature/feature.md:6: error: `work` names `acme/cookbook-docs#99`, which isn't an issue you can read on GitHub\n\n2 nodes: 1 error\n"
    );
    assert_eq!(
        run.stderr,
        "note: skipped checking the work items in `acme/other`: `gh` failed: fake gh: no answer for acme/other#1 on github.com\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn doesnt_call_gh_without_a_github_block() {
    let fixture = tree("gh-no-block", "name: demo\n", "acme/cookbook-docs#41");

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(fixture.gh_calls(), Vec::<String>::new());
}

#[test]
fn only_searches_when_no_node_lists_work() {
    let root = node("fd-000001", "Demo");
    let fixture = Fixture::new(
        "gh-no-work",
        &[("manifestdocs.yaml", GITHUB), ("demo.md", &root)],
    );
    let issues = [(7, "feature: fd-abcdef")];
    fixture.gh_search("github.com", "acme/cookbook-docs", &issues);

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(
        run.stdout,
        "manifestdocs.yaml: warning: issue `acme/cookbook-docs#7` has `feature: fd-abcdef`, which isn't a node in this tree\n\n1 node: 1 warning\n"
    );
    assert_eq!(run.stderr, "");
    let calls = fixture.gh_calls();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].ends_with(
        " -f q=repo:acme/cookbook-docs is:issue in:body \"feature: fd-\" sort:created-asc"
    ));
}

#[test]
fn notes_an_issue_gh_cant_read_and_checks_the_rest() {
    let fixture = tree(
        "gh-forbidden",
        GITHUB,
        "acme/cookbook-docs#5, acme/cookbook-docs#99",
    );
    fixture.gh_unreadable_issue("github.com", "acme/cookbook-docs#5");
    fixture.gh_no_issue("github.com", "acme/cookbook-docs#99");

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(
        run.stdout,
        "feature/feature.md:6: error: `work` names `acme/cookbook-docs#99`, which isn't an issue you can read on GitHub\n\n2 nodes: 1 error\n"
    );
    assert_eq!(
        run.stderr,
        "note: skipped checking `acme/cookbook-docs#5`: Resource not accessible by integration\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn prints_github_findings_as_json() {
    let fixture = tree("gh-json", GITHUB, "acme/cookbook-docs#99");
    fixture.gh_no_issue("github.com", "acme/cookbook-docs#99");
    let issues = [(7, "feature: fd-abcdef"), (8, "feature: search")];
    fixture.gh_search("github.com", "acme/cookbook-docs", &issues);

    let run = fixture.run_with_gh("", &["check", "--json"]);

    let output: Value = serde_json::from_str(&run.stdout).expect("stdout is one JSON object");
    assert_eq!(
        output,
        json!({
            "schema": 1,
            "command": "check",
            "data": {
                "nodes": 2,
                "findings": [
                    {
                        "file": "feature/feature.md",
                        "line": 6,
                        "severity": "error",
                        "code": "issue_not_found",
                        "message": "`work` names `acme/cookbook-docs#99`, which isn't an issue you can read on GitHub"
                    },
                    {
                        "file": "manifestdocs.yaml",
                        "line": null,
                        "severity": "warning",
                        "code": "issue_names_unknown_node",
                        "message": "issue `acme/cookbook-docs#7` has `feature: fd-abcdef`, which isn't a node in this tree"
                    },
                    {
                        "file": "manifestdocs.yaml",
                        "line": null,
                        "severity": "warning",
                        "code": "malformed_feature_line",
                        "message": "issue `acme/cookbook-docs#8` has a malformed `feature:` line `search`: expected `fd-` and 6 lowercase hex characters"
                    }
                ]
            }
        })
    );
    assert_eq!(run.stderr, "");
    assert_eq!(run.code, 1);
}

/// A tree whose plain `check` would ask `gh` about a missing issue and report it.
fn offline_tree(name: &str) -> Fixture {
    let fixture = tree(name, GITHUB, "acme/cookbook-docs#99");
    fixture.gh_no_issue("github.com", "acme/cookbook-docs#99");
    fixture
}

#[test]
fn offline_skips_the_github_rules_without_calling_gh() {
    let fixture = offline_tree("gh-offline");

    let run = fixture.run_with_gh("", &["check", "--offline", "--strict"]);

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(run.stderr, "");
    assert_eq!(run.code, 0);
    assert_eq!(fixture.gh_calls(), Vec::<String>::new());
}

#[test]
fn offline_json_skips_the_github_rules_without_calling_gh() {
    let fixture = offline_tree("gh-offline-json");

    let run = fixture.run_with_gh("", &["check", "--offline", "--json"]);

    let output: Value = serde_json::from_str(&run.stdout).expect("stdout is one JSON object");
    assert_eq!(
        output,
        json!({ "schema": 1, "command": "check", "data": { "nodes": 2, "findings": [] } })
    );
    assert_eq!(run.stderr, "");
    assert_eq!(run.code, 0);
    assert_eq!(fixture.gh_calls(), Vec::<String>::new());
}

#[test]
fn ends_a_repos_search_with_a_note_when_github_repeats_a_cursor() {
    let fixture = tree("gh-search-repeat", GITHUB, "");
    let found = [(7, "feature: fd-000002")];
    fixture.gh_search_page(
        "github.com",
        "acme/cookbook-docs",
        None,
        &found,
        Some("c1"),
        3,
    );
    fixture.gh_search_page(
        "github.com",
        "acme/cookbook-docs",
        Some("c1"),
        &found,
        Some("c1"),
        3,
    );

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(
        run.stderr,
        "note: skipped searching `acme/cookbook-docs` for `feature:` lines: `gh` failed: GitHub repeated the page cursor `c1`\n"
    );
    assert_eq!(run.code, 0);
    assert_eq!(fixture.gh_calls().len(), 2);
}
