//! `manifestdocs context <issue-ref>`.

use std::fs;

use super::{estimate, omitted};
use crate::docs_source::{git as docs_git, workspace};
use crate::support::{Fixture, GhIssue, node};

const GITHUB: &str =
    "name: cookbook\ngithub:\n  host: github.com\n  docs_repo: acme/cookbook-docs\n";

/// A node file that lists `work`.
fn feature(id: &str, title: &str, work: &str, body: &str) -> String {
    format!(
        "---\nid: {id}\ntitle: \"{title}\"\nstatus: draft\nsummary: \"{title} summary.\"\nwork: [{work}]\n---\n\n# {title}\n{body}"
    )
}

/// A tree whose Search node lists `acme/cookbook-docs#41` in `work` and links to a doc, and
/// whose Filters node, Search's child, lists `filters_work`.
fn cookbook(name: &str, filters_work: &str) -> Fixture {
    let search = feature(
        "fd-7c2e41",
        "Search",
        "acme/cookbook-docs#41",
        "\nRanks by the [ranking rules](ranking-rules.md).\n",
    );
    let filters = feature("fd-7c2e99", "Filters", filters_work, "");
    Fixture::new(
        name,
        &[
            ("manifestdocs.yaml", GITHUB),
            ("cookbook.md", &node("fd-0a0001", "Cookbook")),
            ("recipes/recipes.md", &node("fd-0a0002", "Recipes")),
            ("recipes/search/search.md", &search),
            ("recipes/search/ranking-rules.md", "# Ranking rules\n"),
            ("recipes/search/filters/filters.md", &filters),
        ],
    )
}

fn issue(fixture: &Fixture, number: u64, details: GhIssue) {
    let issue = format!("acme/cookbook-docs#{number}");
    fixture.gh_issue_with("github.com", &issue, &details);
}

/// #41 Recipe search > #58 Search filters > #59 Ranking (closed) and #60 Cooking time, with
/// `bodies` for #41 and #58.
fn hierarchy(fixture: &Fixture, bodies: [&str; 2]) {
    issue(
        fixture,
        41,
        GhIssue {
            title: "Recipe search",
            body: bodies[0],
            issue_type: Some("Epic"),
            sub_issues: &["acme/cookbook-docs#58"],
            ..GhIssue::default()
        },
    );
    issue(
        fixture,
        58,
        GhIssue {
            title: "Search filters",
            body: bodies[1],
            parent: Some("acme/cookbook-docs#41"),
            sub_issues: &["acme/cookbook-docs#59", "acme/cookbook-docs#60"],
            ..GhIssue::default()
        },
    );
    let parent = Some("acme/cookbook-docs#58");
    let closed = GhIssue {
        title: "Ranking",
        closed: true,
        parent,
        ..GhIssue::default()
    };
    issue(fixture, 59, closed);
    let task = GhIssue {
        title: "Cooking time",
        body: "Let readers filter by cooking time.\r\n\r\nAcceptance: a slider.\r\n",
        issue_type: Some("Task"),
        parent,
        ..GhIssue::default()
    };
    issue(fixture, 60, task);
}

/// The heading of the node the context is for.
fn target(stdout: &str) -> &str {
    let at = stdout
        .find("\n## Feature: ")
        .map_or("", |at| &stdout[at + 1..]);
    at.lines().next().unwrap_or_default()
}

#[test]
fn assembles_an_issue_with_its_parent_chain_and_linked_node() {
    let fixture = cookbook("context-issue-full", "");
    hierarchy(&fixture, ["", ""]);

    let run = fixture.run_with_gh("", &["context", "acme/cookbook-docs#60"]);

    let expected = "# Context: acme/cookbook-docs#60 (docs not in git)\n\n\
         ## Issue: Cooking time (Task, open)\n\n\
         Let readers filter by cooking time.\n\nAcceptance: a slider.\n\n\
         ## Parent work\n\n\
         - #41 Recipe search (Epic, open, 0 of 1 done)\n  \
           - #58 Search filters (open, 1 of 2 done)\n\n\
         ## Feature: Search (fd-7c2e41, draft)\nPath: recipes/search\n\n\
         # Search\n\nRanks by the [ranking rules](ranking-rules.md).\n\n\
         ## Parent: Recipes (fd-0a0002, draft)\nPath: recipes\n\n# Recipes\n\n\
         ## Project: Cookbook (fd-0a0001, draft)\n\n# Cookbook\n\n\
         ## Linked docs\n\n### recipes/search/ranking-rules.md\n\n# Ranking rules\n\n\
         ## Around this feature\n\n\
         - Child: Filters (fd-7c2e99, draft): Filters summary.\n\n";
    let (before, _) = estimate(&run);
    assert_eq!(before, expected, "{}", run.stderr);
    assert_eq!(run.code, 0);
}

#[test]
fn links_an_issue_and_its_sub_issues_through_a_feature_line() {
    let fixture = cookbook("context-issue-feature-line", "");
    issue(
        &fixture,
        70,
        GhIssue {
            title: "Filter presets",
            body: "Save filters.\n\nfeature: fd-7c2e99\n",
            sub_issues: &["acme/cookbook-docs#71"],
            ..GhIssue::default()
        },
    );
    issue(
        &fixture,
        71,
        GhIssue {
            title: "Name a preset",
            parent: Some("acme/cookbook-docs#70"),
            ..GhIssue::default()
        },
    );

    let issue = fixture.run_with_gh("", &["context", "acme/cookbook-docs#70"]);
    let sub_issue = fixture.run_with_gh("", &["context", "acme/cookbook-docs#71"]);

    assert_eq!(
        target(&issue.stdout),
        "## Feature: Filters (fd-7c2e99, draft)"
    );
    assert_eq!(target(&sub_issue.stdout), target(&issue.stdout));
    assert_eq!((issue.code, sub_issue.code), (0, 0), "{}", sub_issue.stderr);
}

/// #58's `feature:` line is nearer #60 than Search's `work` listing of #41.
#[test]
fn links_an_issue_through_the_nearest_issue_above_it_that_names_a_node() {
    let fixture = cookbook("context-issue-nearest", "");
    hierarchy(&fixture, ["", "feature: fd-7c2e99"]);

    let run = fixture.run_with_gh("", &["context", "acme/cookbook-docs#60"]);

    assert_eq!(
        target(&run.stdout),
        "## Feature: Filters (fd-7c2e99, draft)"
    );
    assert_eq!(run.code, 0, "{}", run.stderr);
}

#[test]
fn prefers_a_nodes_work_to_a_feature_line_on_the_same_issue() {
    let fixture = cookbook("context-issue-work-first", "acme/cookbook-docs#58");
    hierarchy(&fixture, ["", "feature: fd-7c2e41"]);

    let run = fixture.run_with_gh("", &["context", "acme/cookbook-docs#60"]);

    assert_eq!(
        target(&run.stdout),
        "## Feature: Filters (fd-7c2e99, draft)"
    );
    assert_eq!(run.code, 0, "{}", run.stderr);
}

#[test]
fn stops_when_the_issue_links_to_no_node_in_the_tree() {
    let fixture = cookbook("context-issue-unlinked", "");
    let bodies = [
        (80, "No link here.\n\n```\nfeature: fd-7c2e41\n```\n"),
        (81, "feature: fd-ffffff\n"),
        (82, "feature: search\nfeature: fd-7c2e41\n"),
    ];
    for (number, body) in bodies {
        let title = "Loose";
        issue(
            &fixture,
            number,
            GhIssue {
                title,
                body,
                ..GhIssue::default()
            },
        );
    }
    let context =
        |n: u64| fixture.run_with_gh("", &["context", &format!("acme/cookbook-docs#{n}")]);

    let runs = [context(80), context(81), context(82)];

    let stderr: Vec<&str> = runs.iter().map(|run| run.stderr.as_str()).collect();
    assert_eq!(
        stderr,
        [
            "error: issue `acme/cookbook-docs#80` links to no node: no node lists it or a parent \
             in `work`, and none of them has a `feature:` line\n",
            "error: issue `acme/cookbook-docs#81` has `feature: fd-ffffff`, which isn't a node \
             in this tree\n",
            "error: issue `acme/cookbook-docs#82` has a malformed `feature:` line `search`: \
             expected `fd-` and 6 lowercase hex characters\n",
        ]
    );
    for run in &runs {
        assert_eq!((run.code, run.stdout.as_str()), (2, ""));
    }
}

#[test]
fn stops_when_github_cant_return_the_issue() {
    let fixture = cookbook("context-issue-missing", "");
    fixture.gh_no_issue("github.com", "acme/cookbook-docs#99");

    let run = fixture.run_with_gh("", &["context", "acme/cookbook-docs#99"]);

    assert_eq!(
        run.stderr,
        "error: acme/cookbook-docs#99: not an issue you can read on GitHub\n"
    );
    assert_eq!((run.code, run.stdout.as_str()), (2, ""));
}

#[test]
fn needs_gh_and_a_github_block() {
    let fixture = cookbook("context-issue-no-gh", "");
    fs::write(fixture.path("manifestdocs.yaml"), "name: cookbook\n").unwrap();
    let without_block = fixture.run_with_gh("", &["context", "acme/cookbook-docs#60"]);
    fs::write(fixture.path("manifestdocs.yaml"), GITHUB).unwrap();

    let without_gh = fixture.run_without_gh("", &["context", "acme/cookbook-docs#60"]);

    assert_eq!(
        without_block.stderr,
        "error: `context` reads issues from GitHub, and manifestdocs.yaml has no `github` block\n"
    );
    assert_eq!(
        without_gh.stderr,
        "error: `gh` isn't installed; install the GitHub CLI from https://cli.github.com\n"
    );
    assert_eq!((without_block.code, without_gh.code), (2, 2));
}

#[test]
fn marks_a_parent_it_cant_read_and_exits_1() {
    let fixture = cookbook("context-issue-parent-unreadable", "");
    hierarchy(&fixture, ["", ""]);
    fixture.gh_unreadable_issue("github.com", "acme/cookbook-docs#41");

    let run = fixture.run_with_gh("", &["context", "acme/cookbook-docs#60"]);

    let parents = "\n## Parent work\n\n\
         - #41: can't read it on GitHub: Resource not accessible by integration\n  \
           - #58 Search filters (open, 1 of 2 done)\n\n\
         ## Feature: Search (fd-7c2e41, draft)\n";
    assert!(run.stdout.contains(parents), "{}", run.stdout);
    assert_eq!(run.code, 1);
}

#[test]
fn fills_the_budget_with_the_issue_after_linked_docs_and_before_children() {
    let fixture = cookbook("context-issue-budget", "");
    hierarchy(&fixture, ["", ""]);

    let run = fixture.run_with_gh("", &["context", "acme/cookbook-docs#60", "--budget", "1"]);

    assert_eq!(
        omitted(&run),
        [
            "recipes/recipes.md",
            "cookbook.md",
            "recipes/search/ranking-rules.md",
            "acme/cookbook-docs#60",
            "parent work",
            "recipes/search/filters/filters.md",
        ]
    );
    assert!(
        run.stdout
            .contains("\n## Feature: Search (fd-7c2e41, draft)\n")
    );
    assert_eq!(run.code, 0, "{}", run.stderr);
}

/// A factory builds `.context/` from an issue in a code repo, which reads the docs clone's
/// `origin/HEAD` through the pointer.
#[test]
fn writes_an_issues_context_from_origin_head_to_a_file() {
    let (fixture, _, _) = workspace("context-issue-out");
    let docs = "work/docs";
    fs::write(fixture.path("work/docs/manifestdocs.yaml"), GITHUB).unwrap();
    docs_git(&fixture, docs, &["add", "-A"]);
    docs_git(&fixture, docs, &["commit", "-q", "-m", "Link to GitHub"]);
    docs_git(&fixture, docs, &["push", "-q", "origin", "feature/x:main"]);
    docs_git(&fixture, docs, &["fetch", "-q", "origin"]);
    let main = docs_git(&fixture, docs, &["rev-parse", "--short", "origin/main"]);
    let body = "feature: fd-7c2e41\n";
    let title = "Sort by rating";
    issue(
        &fixture,
        7,
        GhIssue {
            title,
            body,
            ..GhIssue::default()
        },
    );
    let out = ".context/context.md";

    let run = fixture.run_with_gh(
        "work/web",
        &["context", "acme/cookbook-docs#7", "--out", out],
    );

    let written = fixture.read("work/web/.context/context.md");
    let header = format!("# Context: acme/cookbook-docs#7 (docs origin/main @ {main}, ");
    assert!(written.starts_with(&header), "{written}");
    assert!(written.contains("\n## Issue: Sort by rating (open)\n\nfeature: fd-7c2e41\n"));
    assert_eq!(run.stderr, format!("Wrote {out}\n"));
    assert_eq!((run.code, run.stdout.as_str()), (0, ""));
}

/// Filters lists #58 but has a bad status, so its errors print, as for `context fd-7c2e99`,
/// rather than Search, which lists #41 above it, standing in.
#[test]
fn prints_the_errors_of_a_broken_node_that_lists_a_parent() {
    let fixture = cookbook("context-issue-broken-node", "acme/cookbook-docs#58");
    let file = fixture.path("recipes/search/filters/filters.md");
    let text = fs::read_to_string(&file).unwrap();
    fs::write(&file, text.replace("status: draft", "status: nonsense")).unwrap();
    hierarchy(&fixture, ["", ""]);

    let run = fixture.run_with_gh("", &["context", "acme/cookbook-docs#60"]);
    let by_id = fixture.run("", &["context", "fd-7c2e99"]);

    assert!(run.stdout.contains("filters.md"), "{}", run.stdout);
    assert_eq!(run.stdout, by_id.stdout);
    assert_eq!((run.code, by_id.code), (1, 1), "{}", run.stderr);
}

#[test]
fn stops_at_a_parent_it_cant_read_when_nothing_below_links_to_a_node() {
    let fixture = cookbook("context-issue-unread-unlinked", "");
    hierarchy(&fixture, ["", ""]);
    // Search lists #41, but the walk can't get past #58 to reach it.
    fixture.gh_unreadable_issue("github.com", "acme/cookbook-docs#58");

    let run = fixture.run_with_gh("", &["context", "acme/cookbook-docs#60"]);

    assert_eq!(
        run.stderr,
        "error: issue `acme/cookbook-docs#60` links to no node below its parent \
         `acme/cookbook-docs#58`, which can't be read: can't read it on GitHub: \
         Resource not accessible by integration\n"
    );
    assert_eq!((run.code, run.stdout.as_str()), (2, ""));
}

#[test]
fn names_a_parent_in_another_repo_in_full() {
    let fixture = cookbook("context-issue-other-repo", "acme/cookbook-docs#58");
    let roadmap = GhIssue {
        title: "Roadmap",
        sub_issues: &["acme/cookbook-docs#58"],
        ..GhIssue::default()
    };
    fixture.gh_issue_with("github.com", "acme/roadmap#1", &roadmap);
    let filters = GhIssue {
        title: "Search filters",
        parent: Some("acme/roadmap#1"),
        sub_issues: &["acme/cookbook-docs#60"],
        ..GhIssue::default()
    };
    issue(&fixture, 58, filters);
    let task = GhIssue {
        title: "Cooking time",
        parent: Some("acme/cookbook-docs#58"),
        ..GhIssue::default()
    };
    issue(&fixture, 60, task);

    let run = fixture.run_with_gh("", &["context", "acme/cookbook-docs#60"]);

    let parents = "\n## Parent work\n\n\
         - acme/roadmap#1 Roadmap (open, 0 of 1 done)\n  \
           - #58 Search filters (open, 0 of 1 done)\n\n";
    assert!(run.stdout.contains(parents), "{}", run.stdout);
    assert_eq!(run.code, 0, "{}", run.stderr);
}

#[test]
fn ref_reads_the_issues_node_at_another_version() {
    let fixture = cookbook("context-issue-ref", "");
    hierarchy(&fixture, ["", ""]);
    super::git(&fixture, &["init", "-q", "-b", "main"]);
    super::git(&fixture, &["add", "-A"]);
    super::git(&fixture, &["commit", "-q", "-m", "Add the cookbook"]);
    let commit = super::git(&fixture, &["rev-parse", "--short", "HEAD"]);
    let file = fixture.path("recipes/search/search.md");
    let text = fs::read_to_string(&file).unwrap();
    fs::write(&file, text.replace("\"Search\"", "\"Search, edited\"")).unwrap();
    let context = |more: &[&str]| {
        let args = [&["context", "acme/cookbook-docs#60"], more].concat();
        fixture.run_with_gh("", &args)
    };

    let edited = context(&[]);
    let on_main = context(&["--ref", "main"]);

    assert_eq!(
        target(&edited.stdout),
        "## Feature: Search, edited (fd-7c2e41, draft)"
    );
    assert_eq!(
        target(&on_main.stdout),
        "## Feature: Search (fd-7c2e41, draft)"
    );
    let header = format!("# Context: acme/cookbook-docs#60 (docs main @ {commit}, 2026-10-27)\n");
    assert!(on_main.stdout.starts_with(&header), "{}", on_main.stdout);
    assert_eq!((edited.code, on_main.code), (0, 0), "{}", on_main.stderr);
}
