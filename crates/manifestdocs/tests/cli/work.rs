//! `manifestdocs work`.

use std::fs;

use serde_json::{Value, json};

use crate::docs_source::git;
use crate::support::{Fixture, GhIssue, node};

const GITHUB: &str = "name: demo\ngithub:\n  host: github.com\n  docs_repo: acme/cookbook-docs\n";

const ISSUE_QUERY: &str = "fragment issue on Issue { number title state issueType { name } \
    parent { number repository { nameWithOwner } } \
    subIssues(first: 100) { pageInfo { hasNextPage endCursor } nodes { number repository { nameWithOwner } } } }";

/// A node file that lists `work`.
fn feature(id: &str, title: &str, work: &str) -> String {
    format!(
        "---\nid: {id}\ntitle: \"{title}\"\nstatus: draft\nsummary: \"{title} summary.\"\nwork: [{work}]\n---\n"
    )
}

/// A tree with a root node and `feature/feature.md`, Feature (fd-000002), which lists `work`.
fn tree(name: &str, work: &str) -> Fixture {
    Fixture::new(
        name,
        &[
            ("manifestdocs.yaml", GITHUB),
            ("demo.md", &node("fd-000001", "Demo")),
            ("feature/feature.md", &feature("fd-000002", "Feature", work)),
        ],
    )
}

/// Has the fake `gh` know `issue` on github.com.
fn issue(fixture: &Fixture, issue: &str, details: GhIssue) {
    fixture.gh_issue_with("github.com", issue, &details);
}

/// The query that reads a batch of issues in one repo, each as `i<number>`.
fn issues_query(numbers: &[u64]) -> String {
    let issues: Vec<String> = numbers
        .iter()
        .map(|n| format!("i{n}: issue(number: {n}) {{ ...issue }}"))
        .collect();
    format!(
        "query($owner: String!, $name: String!) {{ repository(owner: $owner, name: $name) {{ {} }} }} {ISSUE_QUERY}",
        issues.join(" ")
    )
}

fn issues_call(repo: &str, numbers: &[u64]) -> String {
    let (owner, name) = repo.split_once('/').unwrap();
    format!(
        "api graphql --hostname github.com -f query={} -f owner={owner} -f name={name}",
        issues_query(numbers)
    )
}

#[test]
fn lists_work_items_and_their_sub_issues_at_any_depth() {
    let fixture = tree("work-depth", "acme/cookbook-docs#41");
    let docs = |n: u64| format!("acme/cookbook-docs#{n}");
    issue(
        &fixture,
        &docs(41),
        GhIssue {
            title: "Recipe search",
            sub_issues: &["acme/cookbook-docs#58", "acme/cookbook-docs#59"],
            ..GhIssue::default()
        },
    );
    issue(
        &fixture,
        &docs(58),
        GhIssue {
            title: "Search filters",
            parent: Some("acme/cookbook-docs#41"),
            sub_issues: &["acme/cookbook-docs#60"],
            ..GhIssue::default()
        },
    );
    issue(
        &fixture,
        &docs(59),
        GhIssue {
            title: "Ranking",
            parent: Some("acme/cookbook-docs#41"),
            ..GhIssue::default()
        },
    );
    issue(
        &fixture,
        &docs(60),
        GhIssue {
            title: "Cooking time",
            parent: Some("acme/cookbook-docs#58"),
            sub_issues: &["acme/cookbook-docs#61"],
            ..GhIssue::default()
        },
    );
    issue(
        &fixture,
        &docs(61),
        GhIssue {
            title: "Time slider",
            parent: Some("acme/cookbook-docs#60"),
            ..GhIssue::default()
        },
    );

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Feature (fd-000002, draft)\n\
         - acme/cookbook-docs#41 Recipe search (open, 0 of 2 done)\n  \
           - #58 Search filters (open, 0 of 1 done)\n    \
             - #60 Cooking time (open, 0 of 1 done)\n      \
               - #61 Time slider (open)\n  \
           - #59 Ranking (open)\n"
    );
    assert_eq!(run.stderr, "");
    assert_eq!(run.code, 0);
}

#[test]
fn reads_each_level_with_one_read_only_query_per_repo() {
    let fixture = tree("work-calls", "acme/cookbook-docs#41, acme/cookbook-docs#7");
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "Recipe search",
            sub_issues: &["acme/cookbook-docs#58", "acme/cookbook-web#3"],
            ..GhIssue::default()
        },
    );
    fixture.gh_issue("github.com", "acme/cookbook-docs#7");
    fixture.gh_issue("github.com", "acme/cookbook-docs#58");
    fixture.gh_issue("github.com", "acme/cookbook-web#3");

    let run = fixture.run_with_gh("", &["work", "feature"]);

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        fixture.gh_calls(),
        [
            issues_call("acme/cookbook-docs", &[7, 41]),
            issues_call("acme/cookbook-docs", &[58]),
            issues_call("acme/cookbook-web", &[3]),
        ]
    );
}

#[test]
fn names_a_sub_issue_in_another_repo_in_full() {
    let fixture = tree("work-other-repo", "acme/cookbook-docs#41");
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "Recipe search",
            sub_issues: &["acme/cookbook-web#3"],
            ..GhIssue::default()
        },
    );
    fixture.gh_issue("github.com", "acme/cookbook-web#3");

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Feature (fd-000002, draft)\n\
         - acme/cookbook-docs#41 Recipe search (open, 0 of 1 done)\n  \
           - acme/cookbook-web#3 Issue 3 (open)\n"
    );
}

#[test]
fn reads_more_than_100_sub_issues() {
    let fixture = tree("work-pages", "acme/cookbook-docs#41");
    let subs: Vec<String> = (101..=320)
        .map(|n| format!("acme/cookbook-docs#{n}"))
        .collect();
    let subs: Vec<&str> = subs.iter().map(String::as_str).collect();
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "Recipe search",
            sub_issues: &subs,
            ..GhIssue::default()
        },
    );
    for sub in &subs[1..] {
        fixture.gh_issue("github.com", sub);
    }
    issue(
        &fixture,
        subs[0],
        GhIssue {
            title: "First",
            closed: true,
            ..GhIssue::default()
        },
    );

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines.len(), 2 + 220, "{}", run.stderr);
    assert_eq!(
        lines[1],
        "- acme/cookbook-docs#41 Recipe search (open, 1 of 220 done)"
    );
    assert_eq!(lines[2], "  - #101 First (closed)");
    assert_eq!(lines[221], "  - #320 Issue 320 (open)");
    let page = |cursor: &str| {
        format!(
            "api graphql --hostname github.com -f query=query($owner: String!, $name: String!, $cursor: String!) \
             {{ repository(owner: $owner, name: $name) {{ i41: issue(number: 41) {{ \
             subIssues(first: 100, after: $cursor) {{ pageInfo {{ hasNextPage endCursor }} \
             nodes {{ number repository {{ nameWithOwner }} }} }} }} }} }} \
             -f owner=acme -f name=cookbook-docs -f cursor={cursor}"
        )
    };
    let calls = fixture.gh_calls();
    assert_eq!(
        calls[..3],
        [
            issues_call("acme/cookbook-docs", &[41]),
            page("p1"),
            page("p2")
        ]
    );
    // The 220 sub-issues are read in batches of 100.
    assert_eq!(calls.len(), 3 + 3);
}

#[test]
fn leaves_a_sub_issue_to_the_deeper_node_that_lists_it() {
    let fixture = Fixture::new(
        "work-deeper",
        &[
            ("manifestdocs.yaml", GITHUB),
            ("demo.md", &node("fd-000001", "Demo")),
            (
                "recipes/recipes.md",
                &feature("fd-000002", "Recipes", "acme/cookbook-docs#41"),
            ),
            (
                "recipes/search/search.md",
                &feature("fd-000003", "Search", "acme/cookbook-docs#58"),
            ),
        ],
    );
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "Recipes",
            sub_issues: &["acme/cookbook-docs#58", "acme/cookbook-docs#59"],
            ..GhIssue::default()
        },
    );
    issue(
        &fixture,
        "acme/cookbook-docs#58",
        GhIssue {
            title: "Search",
            parent: Some("acme/cookbook-docs#41"),
            sub_issues: &["acme/cookbook-docs#60", "acme/cookbook-docs#61"],
            ..GhIssue::default()
        },
    );
    fixture.gh_issue("github.com", "acme/cookbook-docs#59");
    issue(
        &fixture,
        "acme/cookbook-docs#60",
        GhIssue {
            title: "Filters",
            closed: true,
            ..GhIssue::default()
        },
    );
    fixture.gh_issue("github.com", "acme/cookbook-docs#61");

    let parent = fixture.run_with_gh("", &["work", "fd-000002"]);
    let child = fixture.run_with_gh("", &["work", "fd-000003"]);

    assert_eq!(
        parent.stdout,
        "Recipes (fd-000002, draft)\n\
         - acme/cookbook-docs#41 Recipes (open, 0 of 2 done)\n  \
           - #58 Search (open), listed by Search (fd-000003)\n  \
           - #59 Issue 59 (open)\n"
    );
    assert_eq!(
        child.stdout,
        "Search (fd-000003, draft)\n\
         - acme/cookbook-docs#58 Search (open, 1 of 2 done, sub-issue of #41)\n  \
           - #60 Filters (closed)\n  \
           - #61 Issue 61 (open)\n"
    );
}

#[test]
fn counts_closed_sub_issues_as_done() {
    let fixture = tree("work-counts", "acme/cookbook-docs#41");
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "Recipe search",
            closed: true,
            sub_issues: &[
                "acme/cookbook-docs#1",
                "acme/cookbook-docs#2",
                "acme/cookbook-docs#3",
            ],
            ..GhIssue::default()
        },
    );
    for (n, closed) in [(1, true), (2, false), (3, true)] {
        let title = format!("Step {n}");
        issue(
            &fixture,
            &format!("acme/cookbook-docs#{n}"),
            GhIssue {
                title: &title,
                closed,
                ..GhIssue::default()
            },
        );
    }

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Feature (fd-000002, draft)\n\
         - acme/cookbook-docs#41 Recipe search (closed, 2 of 3 done)\n  \
           - #1 Step 1 (closed)\n  \
           - #2 Step 2 (open)\n  \
           - #3 Step 3 (closed)\n"
    );
}

#[test]
fn labels_issues_with_their_issue_type() {
    let fixture = tree("work-types", "acme/cookbook-docs#41");
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "Recipe search",
            issue_type: Some("Epic"),
            sub_issues: &["acme/cookbook-docs#58"],
            ..GhIssue::default()
        },
    );
    issue(
        &fixture,
        "acme/cookbook-docs#58",
        GhIssue {
            title: "Search filters",
            issue_type: Some("Task"),
            closed: true,
            ..GhIssue::default()
        },
    );

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Feature (fd-000002, draft)\n\
         - acme/cookbook-docs#41 Recipe search (Epic, open, 1 of 1 done)\n  \
           - #58 Search filters (Task, closed)\n"
    );
}

#[test]
fn marks_a_work_item_that_isnt_an_issue_and_exits_1() {
    let fixture = tree(
        "work-not-found",
        "acme/cookbook-docs#41, acme/cookbook-docs#99",
    );
    fixture.gh_issue("github.com", "acme/cookbook-docs#41");
    fixture.gh_no_issue("github.com", "acme/cookbook-docs#99");

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Feature (fd-000002, draft)\n\
         - acme/cookbook-docs#41 Issue 41 (open)\n\
         - acme/cookbook-docs#99: not an issue you can read on GitHub\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn marks_issues_gh_cant_read_with_the_reason_and_exits_1() {
    let fixture = tree(
        "work-forbidden",
        "acme/cookbook-docs#41, acme/cookbook-docs#7",
    );
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "Recipe search",
            sub_issues: &["acme/cookbook-docs#58", "acme/cookbook-docs#59"],
            ..GhIssue::default()
        },
    );
    fixture.gh_unreadable_issue("github.com", "acme/cookbook-docs#7");
    fixture.gh_unreadable_issue("github.com", "acme/cookbook-docs#58");
    fixture.gh_issue("github.com", "acme/cookbook-docs#59");

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Feature (fd-000002, draft)\n\
         - acme/cookbook-docs#41 Recipe search (open, 0 of 2 done)\n  \
           - #58: can't read it on GitHub: Resource not accessible by integration\n  \
           - #59 Issue 59 (open)\n\
         - acme/cookbook-docs#7: can't read it on GitHub: Resource not accessible by integration\n"
    );
    assert_eq!(run.stderr, "");
    assert_eq!(run.code, 1);
}

#[test]
fn shows_the_chain_of_parents_above_a_work_item() {
    let fixture = tree("work-parents", "acme/cookbook-docs#58");
    issue(
        &fixture,
        "acme/cookbook-docs#58",
        GhIssue {
            title: "Search filters",
            parent: Some("acme/cookbook-docs#41"),
            ..GhIssue::default()
        },
    );
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "Recipe search",
            parent: Some("acme/roadmap#1"),
            sub_issues: &["acme/cookbook-docs#58"],
            ..GhIssue::default()
        },
    );
    fixture.gh_issue("github.com", "acme/roadmap#1");

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Feature (fd-000002, draft)\n\
         - acme/cookbook-docs#58 Search filters (open, sub-issue of acme/roadmap#1 > #41)\n"
    );
    let parents_call = |repo: &str, number: u64| {
        let (owner, name) = repo.split_once('/').unwrap();
        format!(
            "api graphql --hostname github.com -f query=query($owner: String!, $name: String!) \
             {{ repository(owner: $owner, name: $name) {{ i{number}: issue(number: {number}) \
             {{ parent {{ number repository {{ nameWithOwner }} }} }} }} }} \
             -f owner={owner} -f name={name}"
        )
    };
    assert_eq!(
        fixture.gh_calls(),
        [
            issues_call("acme/cookbook-docs", &[58]),
            parents_call("acme/cookbook-docs", 41),
            parents_call("acme/roadmap", 1),
        ]
    );
}

#[test]
fn says_when_a_node_has_no_work_items_without_calling_gh() {
    let fixture = Fixture::new(
        "work-none",
        &[
            ("manifestdocs.yaml", GITHUB),
            ("demo.md", &node("fd-000001", "Demo")),
        ],
    );

    let run = fixture.run_with_gh("", &["work", "fd-000001"]);

    assert_eq!(run.stdout, "Demo (fd-000001, draft)\nNo work items.\n");
    assert_eq!(run.code, 0);
    assert!(fixture.gh_calls().is_empty());
}

#[test]
fn fails_with_exit_2_when_gh_is_missing() {
    let fixture = tree("work-no-gh", "acme/cookbook-docs#41");

    let run = fixture.run_without_gh("", &["work", "fd-000002"]);

    assert_eq!(run.stdout, "");
    assert_eq!(
        run.stderr,
        "error: `gh` isn't installed; install the GitHub CLI from https://cli.github.com\n"
    );
    assert_eq!(run.code, 2);
}

#[test]
fn fails_with_exit_2_when_gh_is_logged_out() {
    let fixture = tree("work-logged-out", "acme/cookbook-docs#41");
    fixture.gh_logged_out();

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(run.stdout, "");
    assert_eq!(
        run.stderr,
        "error: `gh` isn't logged in to github.com; run `gh auth login --hostname github.com`\n"
    );
    assert_eq!(run.code, 2);
}

#[test]
fn fails_with_exit_2_in_a_tree_with_no_github_block() {
    let fixture = Fixture::new(
        "work-no-github",
        &[
            ("manifestdocs.yaml", "name: demo\n"),
            ("demo.md", &node("fd-000001", "Demo")),
            (
                "feature/feature.md",
                &feature("fd-000002", "Feature", "acme/cookbook-docs#41"),
            ),
        ],
    );

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(run.stdout, "");
    assert_eq!(
        run.stderr,
        "error: `work` reads issues from GitHub, and manifestdocs.yaml has no `github` block\n"
    );
    assert_eq!(run.code, 2);
    assert!(fixture.gh_calls().is_empty());
}

#[test]
fn prints_a_node_with_frontmatter_errors_and_exits_1() {
    let broken = "---\nid: fd-000002\ntitle: Feature\nstatus: draft\nsummary: \"S.\"\nwork: [acme/cookbook-docs#41]\n---\n";
    let fixture = Fixture::new(
        "work-broken",
        &[
            ("manifestdocs.yaml", GITHUB),
            ("demo.md", &node("fd-000001", "Demo")),
            ("feature/feature.md", broken),
        ],
    );

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "feature/feature.md:3: error: `title` must be double-quoted\n"
    );
    assert_eq!(run.code, 1);
    assert!(fixture.gh_calls().is_empty());
}

#[test]
fn prints_work_as_json() {
    let fixture = Fixture::new(
        "work-json",
        &[
            ("manifestdocs.yaml", GITHUB),
            ("demo.md", &node("fd-000001", "Demo")),
            (
                "recipes/recipes.md",
                &feature(
                    "fd-000002",
                    "Recipes",
                    "acme/cookbook-docs#41, acme/cookbook-docs#99",
                ),
            ),
            (
                "recipes/search/search.md",
                &feature("fd-000003", "Search", "acme/cookbook-docs#59"),
            ),
        ],
    );
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "Recipes",
            issue_type: Some("Epic"),
            parent: Some("acme/roadmap#1"),
            sub_issues: &["acme/cookbook-docs#58", "acme/cookbook-docs#59"],
            ..GhIssue::default()
        },
    );
    issue(
        &fixture,
        "acme/cookbook-docs#58",
        GhIssue {
            title: "Filters",
            closed: true,
            parent: Some("acme/cookbook-docs#41"),
            ..GhIssue::default()
        },
    );
    issue(
        &fixture,
        "acme/cookbook-docs#59",
        GhIssue {
            title: "Search",
            parent: Some("acme/cookbook-docs#41"),
            sub_issues: &["acme/cookbook-docs#60"],
            ..GhIssue::default()
        },
    );
    fixture.gh_no_issue("github.com", "acme/cookbook-docs#99");
    fixture.gh_issue("github.com", "acme/roadmap#1");

    let run = fixture.run_with_gh("recipes", &["work", "fd-000002", "--json"]);

    let output: Value = serde_json::from_str(&run.stdout).expect("stdout is one JSON object");
    let found = |issue: &str, title: &str, state: &str, done: u64, total: u64| {
        json!({
            "issue": issue, "found": true, "error": null, "title": title, "state": state,
            "issue_type": null, "parents": ["acme/cookbook-docs#41", "acme/roadmap#1"],
            "sub_issues_done": done, "sub_issues_total": total, "listed_by": null,
            "sub_issues": [],
        })
    };
    // Search takes #59, so its sub-issues aren't read and its counts are unknown.
    let mut search = found("acme/cookbook-docs#59", "Search", "open", 0, 1);
    search["listed_by"] = json!("fd-000003");
    search["sub_issues_done"] = json!(null);
    search["sub_issues_total"] = json!(null);
    assert_eq!(
        output,
        json!({
            "schema": 1,
            "command": "work",
            "data": {
                "id": "fd-000002",
                "file": "recipes.md",
                "work": [
                    {
                        "issue": "acme/cookbook-docs#41", "found": true, "error": null,
                        "title": "Recipes", "state": "open", "issue_type": "Epic",
                        "parents": ["acme/roadmap#1"],
                        "sub_issues_done": 1, "sub_issues_total": 2, "listed_by": null,
                        "sub_issues": [
                            found("acme/cookbook-docs#58", "Filters", "closed", 0, 0),
                            search,
                        ],
                    },
                    {
                        "issue": "acme/cookbook-docs#99", "found": false,
                        "error": "not an issue you can read on GitHub", "title": null,
                        "state": null, "issue_type": null, "parents": [],
                        "sub_issues_done": null, "sub_issues_total": null, "listed_by": null,
                        "sub_issues": [],
                    },
                ],
                "findings": [],
            },
        })
    );
    assert_eq!(run.code, 1);
}

#[test]
fn prints_a_gh_failure_as_json() {
    let fixture = tree("work-json-error", "acme/cookbook-docs#41");
    fixture.gh_logged_out();

    let run = fixture.run_with_gh("", &["work", "fd-000002", "--json"]);

    let output: Value = serde_json::from_str(&run.stdout).expect("stdout is one JSON object");
    assert_eq!(
        output,
        json!({
            "schema": 1,
            "command": "work",
            "error": "`gh` isn't logged in to github.com; run `gh auth login --hostname github.com`",
        })
    );
    assert_eq!(run.code, 2);
}

#[test]
fn fails_with_exit_2_when_github_rate_limits_the_query() {
    let fixture = tree("work-rate-limited", "acme/cookbook-docs#41");
    fixture.gh_rate_limited();

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(run.stdout, "");
    assert_eq!(run.stderr, "error: `gh` failed: API rate limit exceeded\n");
    assert_eq!(run.code, 2);
}

#[test]
fn fails_with_exit_2_when_github_refuses_the_token() {
    let fixture = tree("work-unauthorized", "acme/cookbook-docs#41");
    fixture.gh_unauthorized();

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stderr,
        "error: `gh` isn't logged in to github.com; run `gh auth login --hostname github.com`\n"
    );
    assert_eq!(run.code, 2);
}

#[test]
fn marks_each_issue_in_a_repo_that_doesnt_exist_and_exits_1() {
    let fixture = tree("work-no-repo", "acme/gone#1, acme/gone#2");
    fixture.gh_no_repo("github.com", "acme/gone");

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Feature (fd-000002, draft)\n\
         - acme/gone#1: not an issue you can read on GitHub\n\
         - acme/gone#2: not an issue you can read on GitHub\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn marks_each_issue_in_a_repo_gh_cant_access_and_exits_1() {
    let fixture = tree(
        "work-forbidden-repo",
        "acme/private#1, acme/cookbook-docs#41",
    );
    fixture.gh_forbidden_repo("github.com", "acme/private");
    fixture.gh_issue("github.com", "acme/cookbook-docs#41");

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Feature (fd-000002, draft)\n\
         - acme/private#1: can't read it on GitHub: Resource protected by organization SAML enforcement.\n\
         - acme/cookbook-docs#41 Issue 41 (open)\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn prints_a_node_with_frontmatter_errors_as_json_with_no_work() {
    let broken = "---\nid: fd-000002\ntitle: Feature\nstatus: draft\nsummary: \"S.\"\nwork: [acme/cookbook-docs#41]\n---\n";
    let fixture = Fixture::new(
        "work-json-broken",
        &[
            ("manifestdocs.yaml", GITHUB),
            ("demo.md", &node("fd-000001", "Demo")),
            ("feature/feature.md", broken),
        ],
    );

    let run = fixture.run_with_gh("", &["work", "fd-000002", "--json"]);

    let output: Value = serde_json::from_str(&run.stdout).expect("stdout is one JSON object");
    assert_eq!(
        output,
        json!({
            "schema": 1,
            "command": "work",
            "data": {
                "id": "fd-000002",
                "file": "feature/feature.md",
                "work": null,
                "findings": [{
                    "file": "feature/feature.md", "line": 3, "severity": "error",
                    "code": "unquoted_field", "message": "`title` must be double-quoted",
                }],
            },
        })
    );
    assert_eq!(run.code, 1);
    assert!(fixture.gh_calls().is_empty());
}

#[test]
fn marks_a_sub_issue_github_cant_return_on_a_later_page() {
    let fixture = tree("work-page-error", "acme/cookbook-docs#41");
    let subs: Vec<String> = (101..=200)
        .map(|n| format!("acme/cookbook-docs#{n}"))
        .collect();
    let mut subs: Vec<&str> = subs.iter().map(String::as_str).collect();
    for sub in &subs {
        fixture.gh_issue("github.com", sub);
    }
    subs.push("-");
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "Recipe search",
            sub_issues: &subs,
            ..GhIssue::default()
        },
    );
    fixture.gh_sub_issue_error("github.com", "acme/cookbook-docs#41", Some("p1"), 0);

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(
        lines[1],
        "- acme/cookbook-docs#41 Recipe search (open, 0 of 101 done)"
    );
    assert_eq!(
        lines.last(),
        Some(
            &"  - unknown sub-issue: can't read it on GitHub: Resource not accessible by integration"
        )
    );
    assert_eq!(lines.len(), 2 + 101, "{}", run.stderr);
    assert_eq!(run.code, 1);
}

#[test]
fn ends_the_parent_chain_at_a_parent_gh_cant_read() {
    let fixture = tree("work-parent-unreadable", "acme/cookbook-docs#58");
    issue(
        &fixture,
        "acme/cookbook-docs#58",
        GhIssue {
            title: "Search filters",
            parent: Some("acme/cookbook-docs#41"),
            ..GhIssue::default()
        },
    );
    fixture.gh_unreadable_issue("github.com", "acme/cookbook-docs#41");

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Feature (fd-000002, draft)\n\
         - acme/cookbook-docs#58 Search filters (open, sub-issue of #41)\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn stops_at_a_sub_issue_cycle() {
    let fixture = tree("work-cycle", "acme/cookbook-docs#41");
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "One",
            parent: Some("acme/cookbook-docs#58"),
            sub_issues: &["acme/cookbook-docs#58"],
            ..GhIssue::default()
        },
    );
    issue(
        &fixture,
        "acme/cookbook-docs#58",
        GhIssue {
            title: "Two",
            parent: Some("acme/cookbook-docs#41"),
            sub_issues: &["acme/cookbook-docs#41"],
            ..GhIssue::default()
        },
    );

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Feature (fd-000002, draft)\n\
         - acme/cookbook-docs#41 One (open, 0 of 1 done, sub-issue of #58)\n  \
           - #58 Two (open, 0 of 1 done)\n    \
             - #41 One (open, 0 of 1 done)\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn matches_repo_names_spelled_with_different_case() {
    let fixture = Fixture::new(
        "work-case",
        &[
            ("manifestdocs.yaml", GITHUB),
            ("demo.md", &node("fd-000001", "Demo")),
            (
                "recipes/recipes.md",
                &feature(
                    "fd-000002",
                    "Recipes",
                    "Acme/Cookbook-Docs#41, acme/cookbook-docs#7",
                ),
            ),
            (
                "recipes/search/search.md",
                &feature("fd-000003", "Search", "ACME/cookbook-docs#58"),
            ),
        ],
    );
    issue(
        &fixture,
        "Acme/Cookbook-Docs#41",
        GhIssue {
            title: "Recipes",
            sub_issues: &["acme/cookbook-docs#58", "acme/cookbook-docs#59"],
            ..GhIssue::default()
        },
    );
    fixture.gh_issue("github.com", "Acme/Cookbook-Docs#7");
    fixture.gh_issue("github.com", "acme/cookbook-docs#58");
    fixture.gh_issue("github.com", "acme/cookbook-docs#59");

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Recipes (fd-000002, draft)\n\
         - Acme/Cookbook-Docs#41 Recipes (open, 0 of 2 done)\n  \
           - #58 Issue 58 (open), listed by Search (fd-000003)\n  \
           - #59 Issue 59 (open)\n\
         - acme/cookbook-docs#7 Issue 7 (open)\n",
        "{}",
        run.stderr
    );
    // One query for each level, since the two spellings name one repo.
    assert_eq!(fixture.gh_calls().len(), 2);
}

#[test]
fn gives_a_sub_issue_two_nodes_list_to_the_shallower_node() {
    let fixture = Fixture::new(
        "work-shallower",
        &[
            ("manifestdocs.yaml", GITHUB),
            (
                "demo.md",
                &feature("fd-000001", "Demo", "acme/cookbook-docs#41"),
            ),
            ("a/a.md", &node("fd-000002", "A")),
            (
                "a/a1/a1.md",
                &feature("fd-000003", "A1", "acme/cookbook-docs#58"),
            ),
            (
                "b/b.md",
                &feature("fd-000004", "B", "acme/cookbook-docs#58"),
            ),
        ],
    );
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "Recipes",
            sub_issues: &["acme/cookbook-docs#58"],
            ..GhIssue::default()
        },
    );
    fixture.gh_issue("github.com", "acme/cookbook-docs#58");

    let run = fixture.run_with_gh("", &["work", "fd-000001"]);

    assert_eq!(
        run.stdout,
        "Demo (fd-000001, draft)\n\
         - acme/cookbook-docs#41 Recipes (open, 0 of 1 done)\n  \
           - #58 Issue 58 (open), listed by B (fd-000004)\n"
    );
}

#[test]
fn shows_a_listed_sub_issue_once_under_its_parent() {
    let fixture = tree(
        "work-listed-twice",
        "acme/cookbook-docs#58, acme/cookbook-docs#41",
    );
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "Recipes",
            sub_issues: &["acme/cookbook-docs#58"],
            ..GhIssue::default()
        },
    );
    issue(
        &fixture,
        "acme/cookbook-docs#58",
        GhIssue {
            title: "Search",
            parent: Some("acme/cookbook-docs#41"),
            ..GhIssue::default()
        },
    );

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Feature (fd-000002, draft)\n\
         - acme/cookbook-docs#41 Recipes (open, 0 of 1 done)\n  \
           - #58 Search (open)\n"
    );
}

#[test]
fn takes_nothing_over_for_a_deeper_node_that_lists_an_issue_above() {
    let fixture = Fixture::new(
        "work-above",
        &[
            ("manifestdocs.yaml", GITHUB),
            ("demo.md", &node("fd-000001", "Demo")),
            (
                "recipes/recipes.md",
                &feature("fd-000002", "Recipes", "acme/cookbook-docs#41"),
            ),
            (
                "recipes/search/search.md",
                &feature("fd-000003", "Search", "acme/cookbook-docs#40"),
            ),
        ],
    );
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "Recipes",
            parent: Some("acme/cookbook-docs#40"),
            sub_issues: &["acme/cookbook-docs#58"],
            ..GhIssue::default()
        },
    );
    fixture.gh_issue("github.com", "acme/cookbook-docs#40");
    fixture.gh_issue("github.com", "acme/cookbook-docs#58");

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Recipes (fd-000002, draft)\n\
         - acme/cookbook-docs#41 Recipes (open, 0 of 1 done, sub-issue of #40)\n  \
           - #58 Issue 58 (open)\n"
    );
}

/// A workspace with a code repo, `work/web`, and a docs clone, `work/docs`, reached through a
/// pointer. Feature lists #41 on `main`, which is `origin/HEAD`, and #42 on the checked-out
/// `feature/x` branch.
fn workspace(name: &str) -> Fixture {
    let fixture = Fixture::new(
        name,
        &[
            ("work/.manifestdocs/config.yaml", "tree: docs\n"),
            ("work/web/src/main.rs", "fn main() {}\n"),
            ("work/docs/manifestdocs.yaml", GITHUB),
            ("work/docs/demo.md", &node("fd-000001", "Demo")),
            (
                "work/docs/feature/feature.md",
                &feature("fd-000002", "Feature", "acme/cookbook-docs#41"),
            ),
        ],
    );
    let docs = "work/docs";
    git(&fixture, "work/web", &["init", "-q"]);
    git(&fixture, "", &["init", "-q", "--bare", "origin.git"]);
    git(&fixture, docs, &["init", "-q", "-b", "main"]);
    git(&fixture, docs, &["add", "-A"]);
    git(&fixture, docs, &["commit", "-q", "-m", "Add the tree"]);
    git(
        &fixture,
        docs,
        &["remote", "add", "origin", "../../origin.git"],
    );
    git(&fixture, docs, &["push", "-q", "origin", "main"]);
    git(&fixture, docs, &["remote", "set-head", "origin", "main"]);
    git(&fixture, docs, &["checkout", "-q", "-b", "feature/x"]);
    let moved = feature("fd-000002", "Feature", "acme/cookbook-docs#42");
    fs::write(fixture.path("work/docs/feature/feature.md"), moved).unwrap();
    git(&fixture, docs, &["commit", "-q", "-am", "Move the work"]);
    fixture.gh_issue("github.com", "acme/cookbook-docs#41");
    fixture.gh_issue("github.com", "acme/cookbook-docs#42");
    fixture
}

#[test]
fn reads_origin_head_through_a_pointer() {
    let fixture = workspace("work-pointer");

    let run = fixture.run_with_gh("work/web/src", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout, "Feature (fd-000002, draft)\n- acme/cookbook-docs#41 Issue 41 (open)\n",
        "{}",
        run.stderr
    );
    assert_eq!(run.code, 0);
}

#[test]
fn reads_the_ref_given() {
    let fixture = workspace("work-ref");

    let working_tree = fixture.run_with_gh("work/docs", &["work", "fd-000002"]);
    let main = fixture.run_with_gh("work/docs", &["work", "fd-000002", "--ref", "main"]);

    assert_eq!(
        working_tree.stdout,
        "Feature (fd-000002, draft)\n- acme/cookbook-docs#42 Issue 42 (open)\n",
        "{}",
        working_tree.stderr
    );
    assert_eq!(
        main.stdout, "Feature (fd-000002, draft)\n- acme/cookbook-docs#41 Issue 41 (open)\n",
        "{}",
        main.stderr
    );
}

#[test]
fn reads_only_the_first_fetch_of_a_taken_over_issue() {
    let fixture = Fixture::new(
        "work-taken-over-pages",
        &[
            ("manifestdocs.yaml", GITHUB),
            ("demo.md", &node("fd-000001", "Demo")),
            (
                "recipes/recipes.md",
                &feature("fd-000002", "Recipes", "acme/cookbook-docs#41"),
            ),
            (
                "recipes/search/search.md",
                &feature("fd-000003", "Search", "acme/cookbook-docs#58"),
            ),
        ],
    );
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "Recipes",
            sub_issues: &["acme/cookbook-docs#58"],
            ..GhIssue::default()
        },
    );
    let subs: Vec<String> = (101..=250)
        .map(|n| format!("acme/cookbook-docs#{n}"))
        .collect();
    let subs: Vec<&str> = subs.iter().map(String::as_str).collect();
    issue(
        &fixture,
        "acme/cookbook-docs#58",
        GhIssue {
            title: "Search",
            sub_issues: &subs,
            ..GhIssue::default()
        },
    );

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Recipes (fd-000002, draft)\n\
         - acme/cookbook-docs#41 Recipes (open, 0 of 1 done)\n  \
           - #58 Search (open), listed by Search (fd-000003)\n",
        "{}",
        run.stderr
    );
    assert_eq!(
        fixture.gh_calls(),
        [
            issues_call("acme/cookbook-docs", &[41]),
            issues_call("acme/cookbook-docs", &[58]),
        ]
    );
}

#[test]
fn marks_the_sub_issues_past_a_repeated_cursor_and_exits_1() {
    let fixture = tree("work-repeated-cursor", "acme/cookbook-docs#41");
    let subs: Vec<String> = (101..=250)
        .map(|n| format!("acme/cookbook-docs#{n}"))
        .collect();
    let subs: Vec<&str> = subs.iter().map(String::as_str).collect();
    for sub in &subs {
        fixture.gh_issue("github.com", sub);
    }
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "Recipe search",
            sub_issues: &subs,
            ..GhIssue::default()
        },
    );
    fixture.gh_repeat_sub_issue_cursor("github.com", "acme/cookbook-docs#41", "p1");

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(
        lines[1],
        "- acme/cookbook-docs#41 Recipe search (open, 0 of 150 done)"
    );
    assert_eq!(
        lines.last(),
        Some(&"  - sub-issues past 150 couldn't be read, since GitHub repeated a page cursor")
    );
    assert_eq!(lines.len(), 2 + 150 + 1, "{}", run.stderr);
    assert_eq!(run.code, 1);
    let pages = fixture.gh_calls();
    let pages = pages.iter().filter(|call| call.contains("-f cursor=p1"));
    assert_eq!(pages.count(), 1);
}

#[test]
fn keeps_ghs_message_when_its_answer_has_no_graphql_errors() {
    let fixture = tree("work-http-error", "acme/cookbook-docs#41");
    fixture.gh_http_error(
        r#"{"message":"API rate limit exceeded for user ID 1."}"#,
        "gh: API rate limit exceeded for user ID 1. (HTTP 403)",
    );

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(run.stdout, "");
    assert_eq!(
        run.stderr,
        "error: `gh` failed: gh: API rate limit exceeded for user ID 1. (HTTP 403)\n"
    );
    assert_eq!(run.code, 2);
}

#[test]
fn marks_a_sub_issue_github_cant_return_on_the_first_page() {
    let fixture = tree("work-first-page-error", "acme/cookbook-docs#41");
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "Recipe search",
            sub_issues: &["acme/cookbook-docs#58", "-"],
            ..GhIssue::default()
        },
    );
    issue(
        &fixture,
        "acme/cookbook-docs#58",
        GhIssue {
            title: "Filters",
            closed: true,
            ..GhIssue::default()
        },
    );
    fixture.gh_sub_issue_error("github.com", "acme/cookbook-docs#41", None, 1);

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Feature (fd-000002, draft)\n\
         - acme/cookbook-docs#41 Recipe search (open, 1 of 2 done)\n  \
           - #58 Filters (closed)\n  \
           - unknown sub-issue: can't read it on GitHub: Resource not accessible by integration\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn ends_the_parent_chain_at_a_repo_gh_cant_access() {
    let fixture = tree("work-parent-repo-forbidden", "acme/cookbook-docs#58");
    issue(
        &fixture,
        "acme/cookbook-docs#58",
        GhIssue {
            title: "Search filters",
            parent: Some("acme/roadmap#1"),
            ..GhIssue::default()
        },
    );
    fixture.gh_forbidden_repo("github.com", "acme/roadmap");

    let run = fixture.run_with_gh("", &["work", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Feature (fd-000002, draft)\n\
         - acme/cookbook-docs#58 Search filters (open, sub-issue of acme/roadmap#1)\n"
    );
    assert_eq!(run.stderr, "");
    assert_eq!(run.code, 0);
}

#[test]
fn gives_a_sub_issue_two_nodes_at_one_depth_list_to_the_first_in_outline_order() {
    let fixture = Fixture::new(
        "work-tie",
        &[
            ("manifestdocs.yaml", GITHUB),
            (
                "demo.md",
                &feature("fd-000001", "Demo", "acme/cookbook-docs#41"),
            ),
            (
                "b/b.md",
                &feature("fd-000003", "B", "acme/cookbook-docs#58"),
            ),
            (
                "a/a.md",
                &feature("fd-000002", "A", "acme/cookbook-docs#58"),
            ),
        ],
    );
    issue(
        &fixture,
        "acme/cookbook-docs#41",
        GhIssue {
            title: "Recipes",
            sub_issues: &["acme/cookbook-docs#58"],
            ..GhIssue::default()
        },
    );
    fixture.gh_issue("github.com", "acme/cookbook-docs#58");

    let run = fixture.run_with_gh("", &["work", "fd-000001"]);

    assert_eq!(
        run.stdout,
        "Demo (fd-000001, draft)\n\
         - acme/cookbook-docs#41 Recipes (open, 0 of 1 done)\n  \
           - #58 Issue 58 (open), listed by A (fd-000002)\n"
    );
}
