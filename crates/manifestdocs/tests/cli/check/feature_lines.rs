use crate::support::{Fixture, node};

const GITHUB: &str = "name: demo\ngithub:\n  host: github.com\n  docs_repo: acme/cookbook-docs\n";

/// A tree with a root node and `feature/feature.md` (`fd-000002`), which lists `repos` and
/// one issue in `work` that exists.
fn tree(name: &str, repos: &str) -> Fixture {
    let root = node("fd-000001", "Demo");
    let feature = format!(
        "---\nid: fd-000002\ntitle: \"Feature\"\nstatus: draft\nsummary: \"Feature summary.\"\nrepos: [{repos}]\nwork: [acme/cookbook-docs#41]\n---\n"
    );
    let fixture = Fixture::new(
        name,
        &[
            ("manifestdocs.yaml", GITHUB),
            ("demo.md", &root),
            ("feature/feature.md", &feature),
        ],
    );
    fixture.gh_issue("github.com", "acme/cookbook-docs#41");
    fixture
}

#[test]
fn warns_about_an_issue_that_names_a_node_not_in_the_tree() {
    let fixture = tree("feature-unknown", "");
    let issues = [
        (7, "Fix ranking.\n\nfeature: fd-abcdef\n"),
        (8, "feature: fd-000002\n"),
        (9, "No link here.\n"),
    ];
    fixture.gh_search("github.com", "acme/cookbook-docs", &issues);

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(
        run.stdout,
        "manifestdocs.yaml: warning: issue `acme/cookbook-docs#7` has `feature: fd-abcdef`, which isn't a node in this tree\n\n2 nodes: 1 warning\n"
    );
    assert_eq!(run.stderr, "");
    assert_eq!(run.code, 0);
}

#[test]
fn warns_about_a_malformed_feature_line() {
    let fixture = tree("feature-malformed", "");
    let issues = [(7, "Intro.\r\n\r\nfeature: search page\r\n")];
    fixture.gh_search("github.com", "acme/cookbook-docs", &issues);

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(
        run.stdout,
        "manifestdocs.yaml: warning: issue `acme/cookbook-docs#7` has a malformed `feature:` line `search page`: expected `fd-` and 6 lowercase hex characters\n\n2 nodes: 1 warning\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn searches_the_docs_repo_and_every_repo_a_node_lists() {
    let fixture = tree("feature-repos", "acme/web, acme/api, acme/cookbook-docs");
    fixture.gh_search("github.com", "acme/api", &[(3, "feature: fd-abcdef")]);
    fixture.gh_search("github.com", "acme/cookbook-docs", &[]);
    fixture.gh_search("github.com", "acme/web", &[(5, "feature: fd-000002")]);

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(
        run.stdout,
        "manifestdocs.yaml: warning: issue `acme/api#3` has `feature: fd-abcdef`, which isn't a node in this tree\n\n2 nodes: 1 warning\n"
    );
    let searches: Vec<String> = fixture
        .gh_calls()
        .iter()
        .filter_map(|call| call.split_once(" -f q=").map(|(_, q)| q.to_owned()))
        .collect();
    assert_eq!(
        searches,
        [
            "repo:acme/api is:issue in:body \"feature: fd-\" sort:created-asc",
            "repo:acme/cookbook-docs is:issue in:body \"feature: fd-\" sort:created-asc",
            "repo:acme/web is:issue in:body \"feature: fd-\" sort:created-asc",
        ]
    );
}

#[test]
fn reads_every_page_of_search_results() {
    let fixture = tree("feature-pages", "");
    let first = [(7, "feature: fd-000002")];
    let repo = "acme/cookbook-docs";
    fixture.gh_search_page("github.com", repo, None, &first, Some("c2"), 2);
    let second = [(12, "feature: fd-abcdef")];
    fixture.gh_search_page("github.com", repo, Some("c2"), &second, None, 2);

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(
        run.stdout,
        "manifestdocs.yaml: warning: issue `acme/cookbook-docs#12` has `feature: fd-abcdef`, which isn't a node in this tree\n\n2 nodes: 1 warning\n"
    );
    assert_eq!(run.stderr, "");
    assert!(fixture.gh_calls()[2].ends_with(" -f cursor=c2"));
}

#[test]
fn skips_searching_only_the_repo_gh_fails_for() {
    let fixture = tree("feature-search-fails", "acme/web");
    fixture.gh_search(
        "github.com",
        "acme/cookbook-docs",
        &[(7, "feature: fd-abcdef")],
    );

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(
        run.stdout,
        "manifestdocs.yaml: warning: issue `acme/cookbook-docs#7` has `feature: fd-abcdef`, which isn't a node in this tree\n\n2 nodes: 1 warning\n"
    );
    assert_eq!(
        run.stderr,
        "note: skipped searching `acme/web` for `feature:` lines: `gh` failed: fake gh: no search results for acme/web on github.com\n"
    );
}

const UNKNOWN_7: &str = "manifestdocs.yaml: warning: issue `acme/cookbook-docs#7` has `feature: fd-abcdef`, which isn't a node in this tree\n\n2 nodes: 1 warning\n";

#[test]
fn leaves_out_a_search_result_thats_a_pull_request() {
    let fixture = tree("feature-pull-request", "");
    let answer = r#"{"data":{"search":{"issueCount":2,"pageInfo":{"hasNextPage":false,"endCursor":null},
        "nodes":[{},{"number":7,"body":"feature: fd-abcdef"}]}}}"#;
    fixture.gh_search_answer("github.com", "acme/cookbook-docs", None, answer);

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(run.stdout, UNKNOWN_7);
    assert_eq!(run.stderr, "");
}

#[test]
fn skips_a_search_whose_answer_has_errors() {
    let fixture = tree("feature-search-errors", "");
    let answer = r#"{"data":{"search":{"issueCount":1,"pageInfo":{"hasNextPage":false,"endCursor":null},
        "nodes":[{"number":7,"body":"feature: fd-abcdef"}]}},
        "errors":[{"type":"TIMEOUT","message":"Search timed out"}]}"#;
    fixture.gh_search_answer("github.com", "acme/cookbook-docs", None, answer);

    let run = fixture.run_with_gh("", &["check", "--strict"]);

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(
        run.stderr,
        "note: skipped searching `acme/cookbook-docs` for `feature:` lines: `gh` failed: Search timed out\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn warns_once_about_an_issue_the_search_returns_twice() {
    let fixture = tree("feature-twice", "");
    let issue = [(7, "feature: fd-abcdef")];
    let repo = "acme/cookbook-docs";
    fixture.gh_search_page("github.com", repo, None, &issue, Some("c2"), 2);
    fixture.gh_search_page("github.com", repo, Some("c2"), &issue, None, 2);

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(run.stdout, UNKNOWN_7);
}

#[test]
fn notes_a_search_cut_short_by_githubs_limit() {
    let fixture = tree("feature-cut-short", "");
    let issue = [(7, "feature: fd-abcdef")];
    fixture.gh_search_page("github.com", "acme/cookbook-docs", None, &issue, None, 1500);

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(run.stdout, UNKNOWN_7);
    assert_eq!(
        run.stderr,
        "note: the search of `acme/cookbook-docs` for `feature:` lines stopped at 1 of 1500 issues, since GitHub search returns at most 1,000\n"
    );
}

#[test]
fn searches_a_repo_listed_in_two_cases_once() {
    let fixture = tree(
        "feature-repo-case",
        "Acme/web, acme/web, acme/Cookbook-Docs",
    );
    fixture.gh_search("github.com", "acme/cookbook-docs", &[]);
    fixture.gh_search("github.com", "Acme/web", &[]);

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    let searches: Vec<String> = fixture
        .gh_calls()
        .iter()
        .filter_map(|call| call.split_once(" -f q=").map(|(_, q)| q.to_owned()))
        .collect();
    assert_eq!(
        searches,
        [
            "repo:Acme/web is:issue in:body \"feature: fd-\" sort:created-asc",
            "repo:acme/cookbook-docs is:issue in:body \"feature: fd-\" sort:created-asc",
        ]
    );
}
