//! `manifestdocs stale`.

use std::fs;
use std::process::Command;

use serde_json::{Value, json};

use crate::support::{Fixture, GhIssue, node};

const CONFIG: &str = "name: demo\ngithub:\n  host: github.com\n  docs_repo: acme/docs\n";

/// When the docs are first committed.
const FIRST: &str = "2026-10-01T10:00:00Z";
/// When a test changes the docs.
const LATER: &str = "2026-10-10T10:00:00Z";

/// A node file that lists `work`.
fn feature(id: &str, title: &str, work: &str) -> String {
    format!(
        "---\nid: {id}\ntitle: \"{title}\"\nstatus: draft\nsummary: \"{title} summary.\"\nwork: [{work}]\n---\n\n# {title}\n"
    )
}

/// A tree in `folder`: a root node; Search (fd-000002), which lists `work` and owns an image,
/// a decision records folder and a nested tree; and Search's child Filters (fd-000003),
/// which lists `filters_work`.
fn files(folder: &str, work: &str, filters_work: &str) -> Vec<(String, String)> {
    let files = [
        ("manifestdocs.yaml", CONFIG.to_owned()),
        ("demo.md", node("fd-000001", "Demo")),
        ("search/search.md", feature("fd-000002", "Search", work)),
        ("search/images/flow.png", "\0png 1".to_owned()),
        (
            "search/decisions/2026-10-01-ranking.md",
            "# Ranking\n".to_owned(),
        ),
        (
            "search/filters/filters.md",
            feature("fd-000003", "Filters", filters_work),
        ),
        (
            "search/vendor/manifestdocs.yaml",
            "name: vendor\n".to_owned(),
        ),
        ("search/vendor/vendor.md", node("fd-000009", "Vendor")),
    ];
    let files = files.into_iter();
    files
        .map(|(path, text)| (format!("{folder}{path}"), text))
        .collect()
}

fn fixture(name: &str, files: &[(String, String)]) -> Fixture {
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    Fixture::new(name, &files)
}

/// The tree at the fixture's root, committed on `FIRST` and pushed, with no issues that name
/// a node in a `feature:` line. Returns the commit.
fn docs(name: &str, work: &str, filters_work: &str) -> (Fixture, String) {
    docs_with(name, &files("", work, filters_work))
}

/// The tree in `files` at the fixture's root, as `docs` commits it.
fn docs_with(name: &str, files: &[(String, String)]) -> (Fixture, String) {
    let fixture = fixture(name, files);
    git(&fixture, "", &["init", "-q", "-b", "main"], FIRST);
    let first = publish(&fixture, "", FIRST);
    fixture.gh_search("github.com", "acme/docs", &[]);
    (fixture, first)
}

/// Runs `git` in `folder` at `date`, with its own home folder so no user config applies.
fn git(fixture: &Fixture, folder: &str, args: &[&str], date: &str) -> String {
    let output = Command::new("git")
        .args(["-c", "user.name=Sam", "-c", "user.email=sam@example.com"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(fixture.path(folder))
        .env("HOME", fixture.path(""))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .output()
        .expect("git runs");
    assert!(output.status.success(), "git {args:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// Commits everything in the repo in `folder` at `date`, returning the commit.
fn commit(fixture: &Fixture, folder: &str, date: &str) -> String {
    git(fixture, folder, &["add", "-A"], date);
    git(
        fixture,
        folder,
        &["commit", "-q", "-m", "Change the docs"],
        date,
    );
    git(fixture, folder, &["rev-parse", "HEAD"], date)
}

/// Commits as `commit` does, and moves `origin/HEAD`, as `origin/main`, to the commit, as if
/// it were pushed and fetched.
fn publish(fixture: &Fixture, folder: &str, date: &str) -> String {
    let commit = commit(fixture, folder, date);
    push(fixture, folder, &commit);
    commit
}

/// Moves `origin/HEAD`, as `origin/main`, to `commit`, as if it were pushed and fetched.
fn push(fixture: &Fixture, folder: &str, commit: &str) {
    let update = ["update-ref", "refs/remotes/origin/main", commit];
    git(fixture, folder, &update, FIRST);
    let origin_head = ["refs/remotes/origin/HEAD", "refs/remotes/origin/main"];
    let symbolic_ref = [&["symbolic-ref"][..], &origin_head].concat();
    git(fixture, folder, &symbolic_ref, FIRST);
}

fn short(fixture: &Fixture, folder: &str, commit: &str) -> String {
    git(fixture, folder, &["rev-parse", "--short", commit], FIRST)
}

/// Adds a line to Search's node file, which still lists `work`.
fn edit_search(fixture: &Fixture, folder: &str, work: &str) {
    let text = format!("{}More.\n", feature("fd-000002", "Search", work));
    fs::write(fixture.path(&format!("{folder}search/search.md")), text).unwrap();
}

/// Has the fake `gh` know `acme/docs#number`.
fn issue(fixture: &Fixture, number: u64, details: GhIssue) {
    fixture.gh_issue_with("github.com", &format!("acme/docs#{number}"), &details);
}

fn planned_against(commit: &str) -> String {
    format!("Plan the search.\n\nspec-ref: {commit}\n")
}

#[test]
fn reports_an_open_issue_whose_node_changed_after_its_spec_ref() {
    let (fixture, first) = docs("stale-spec-ref", "acme/docs#41", "");
    edit_search(&fixture, "", "acme/docs#41");
    fs::write(fixture.path("search/images/flow.png"), "\0png 2").unwrap();
    let later = publish(&fixture, "", LATER);
    // A short ID works as a spec-ref.
    let body = planned_against(&short(&fixture, "", &first));
    issue(
        &fixture,
        41,
        GhIssue {
            title: "Recipe search",
            body: &body,
            ..GhIssue::default()
        },
    );

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(
        run.stdout,
        format!(
            "acme/docs#41 Recipe search\n  \
             Search (fd-000002, draft)  search/search.md\n  \
             changed in {} (2026-10-10) after its spec-ref {} (2026-10-01)\n    \
             search/images/flow.png (binary)\n    \
             search/search.md (+1 -0)\n\
             \n\
             1 open issue: 1 stale\n",
            short(&fixture, "", &later),
            short(&fixture, "", &first)
        )
    );
    assert_eq!(run.stderr, "");
    assert_eq!(run.code, 1);
}

#[test]
fn passes_an_issue_planned_against_the_nodes_latest_change() {
    let (fixture, _) = docs("stale-current", "acme/docs#41", "");
    edit_search(&fixture, "", "acme/docs#41");
    let later = publish(&fixture, "", LATER);
    let body = planned_against(&later);
    issue(
        &fixture,
        41,
        GhIssue {
            title: "Recipe search",
            body: &body,
            ..GhIssue::default()
        },
    );

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(run.stdout, "1 open issue: none stale\n");
    assert_eq!(run.code, 0);
}

#[test]
fn skips_closed_issues() {
    let (fixture, first) = docs("stale-closed", "acme/docs#41", "");
    edit_search(&fixture, "", "acme/docs#41");
    publish(&fixture, "", LATER);
    let body = planned_against(&first);
    issue(
        &fixture,
        41,
        GhIssue {
            title: "Recipe search",
            closed: true,
            body: &body,
            ..GhIssue::default()
        },
    );

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(run.stdout, "0 open issues: none stale\n");
    assert_eq!(run.code, 0);
}

#[test]
fn falls_back_to_the_default_branch_when_the_issue_was_created() {
    let (fixture, first) = docs("stale-created", "acme/docs#41, acme/docs#42", "");
    edit_search(&fixture, "", "acme/docs#41, acme/docs#42");
    let later = publish(&fixture, "", LATER);
    let created = |number, title, created_at| {
        let details = GhIssue {
            title,
            created_at: Some(created_at),
            ..GhIssue::default()
        };
        issue(&fixture, number, details);
    };
    created(41, "Recipe search", "2026-10-05T09:00:00Z");
    created(42, "Ranking", "2026-10-12T09:00:00Z");

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(
        run.stdout,
        format!(
            "acme/docs#41 Recipe search\n  \
             Search (fd-000002, draft)  search/search.md\n  \
             changed in {} (2026-10-10) after {} (2026-10-01), the docs when it was created\n    \
             search/search.md (+1 -0)\n\
             \n\
             2 open issues: 1 stale\n",
            short(&fixture, "", &later),
            short(&fixture, "", &first)
        )
    );
    assert_eq!(run.code, 1);
}

#[test]
fn counts_every_change_for_an_issue_older_than_the_docs() {
    let (fixture, first) = docs("stale-older", "acme/docs#41", "");
    let details = GhIssue {
        title: "Recipe search",
        created_at: Some("2026-09-01T09:00:00Z"),
        ..GhIssue::default()
    };
    issue(&fixture, 41, details);

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(
        run.stdout,
        format!(
            "acme/docs#41 Recipe search\n  \
             Search (fd-000002, draft)  search/search.md\n  \
             changed in {} (2026-10-01); the issue is older than every docs commit\n    \
             search/images/flow.png (binary)\n    \
             search/search.md (+9 -0)\n\
             \n\
             1 open issue: 1 stale\n",
            short(&fixture, "", &first)
        )
    );
    assert_eq!(run.code, 1);
}

#[test]
fn watches_only_the_files_a_node_owns() {
    let (fixture, first) = docs("stale-owned", "acme/docs#41", "");
    let filters = format!("{}More.\n", feature("fd-000003", "Filters", ""));
    fs::write(fixture.path("search/filters/filters.md"), filters).unwrap();
    let record = "search/decisions/2026-10-01-ranking.md";
    fs::write(fixture.path(record), "# Ranking, edited\n").unwrap();
    fs::write(fixture.path("demo.md"), node("fd-000001", "Demo, edited")).unwrap();
    let nested = node("fd-000009", "Vendor, edited");
    fs::write(fixture.path("search/vendor/vendor.md"), nested).unwrap();
    publish(&fixture, "", LATER);
    let body = planned_against(&first);
    issue(
        &fixture,
        41,
        GhIssue {
            title: "Recipe search",
            body: &body,
            ..GhIssue::default()
        },
    );

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(run.stdout, "1 open issue: none stale\n");
    assert_eq!(run.code, 0);
}

#[test]
fn checks_sub_issues_and_issues_that_name_the_node_in_a_feature_line() {
    let (fixture, first) = docs("stale-links", "acme/docs#41", "");
    edit_search(&fixture, "", "acme/docs#41");
    let later = publish(&fixture, "", LATER);
    let current = planned_against(&later);
    let old = planned_against(&first);
    let named = format!("feature: fd-000002\n{old}");
    fixture.gh_search("github.com", "acme/docs", &[(70, &named)]);
    let details = |title, body, sub_issues, parent| GhIssue {
        title,
        body,
        sub_issues,
        parent,
        ..GhIssue::default()
    };
    issue(
        &fixture,
        41,
        details("Recipe search", &current, &["acme/docs#58"], None),
    );
    issue(
        &fixture,
        58,
        details("Search filters", &old, &[], Some("acme/docs#41")),
    );
    issue(&fixture, 70, details("Search speed", &named, &[], None));

    let run = fixture.run_with_gh("", &["stale"]);

    let block = |issue: &str| {
        format!(
            "{issue}\n  \
             Search (fd-000002, draft)  search/search.md\n  \
             changed in {} (2026-10-10) after its spec-ref {} (2026-10-01)\n    \
             search/search.md (+1 -0)\n",
            short(&fixture, "", &later),
            short(&fixture, "", &first)
        )
    };
    assert_eq!(
        run.stdout,
        format!(
            "{}\n{}\n3 open issues: 2 stale\n",
            block("acme/docs#58 Search filters"),
            block("acme/docs#70 Search speed")
        )
    );
    assert_eq!(run.code, 1);
}

#[test]
fn applies_a_sub_issue_that_a_deeper_node_lists_to_that_node() {
    let (fixture, first) = docs("stale-deeper", "acme/docs#41", "acme/docs#58");
    edit_search(&fixture, "", "acme/docs#41");
    let later = publish(&fixture, "", LATER);
    let current = planned_against(&later);
    let old = planned_against(&first);
    issue(
        &fixture,
        41,
        GhIssue {
            title: "Recipe search",
            body: &current,
            sub_issues: &["acme/docs#58"],
            ..GhIssue::default()
        },
    );
    issue(
        &fixture,
        58,
        GhIssue {
            title: "Search filters",
            body: &old,
            parent: Some("acme/docs#41"),
            ..GhIssue::default()
        },
    );

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(run.stdout, "2 open issues: none stale\n");
    assert_eq!(run.code, 0);
}

#[test]
fn reports_a_spec_ref_that_isnt_a_commit_in_the_docs_clone() {
    let (fixture, _) = docs("stale-bad-ref", "acme/docs#41", "");
    issue(
        &fixture,
        41,
        GhIssue {
            title: "Recipe search",
            body: "spec-ref: 0000000\n",
            ..GhIssue::default()
        },
    );

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(
        run.stdout,
        "acme/docs#41 Recipe search\n  \
         Search (fd-000002, draft)  search/search.md\n  \
         can't check it: its spec-ref `0000000` isn't a commit in the docs clone\n\
         \n\
         1 open issue: none stale, 1 not checked\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn marks_a_linked_issue_gh_cant_read() {
    let (fixture, _) = docs("stale-unreadable", "acme/docs#41", "");
    fixture.gh_unreadable_issue("github.com", "acme/docs#41");

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(
        run.stdout,
        "acme/docs#41\n  \
         Search (fd-000002, draft)  search/search.md\n  \
         can't read it on GitHub: Resource not accessible by integration\n\
         \n\
         0 open issues: none stale, 1 not checked\n"
    );
    assert_eq!(run.code, 1);
}

/// From another repo, `stale` reads the docs clone's `origin/HEAD`, as `work` does, so a
/// change on the branch the clone has checked out doesn't count until it merges.
#[test]
fn reads_the_default_branch_from_another_repo() {
    let mut files = files("docs/", "acme/docs#41", "");
    files.push((
        "web/.manifestdocs/config.yaml".into(),
        "tree: ../docs\n".into(),
    ));
    let fixture = fixture("stale-pointer", &files);
    git(&fixture, "web", &["init", "-q"], FIRST);
    git(&fixture, "docs", &["init", "-q", "-b", "main"], FIRST);
    let first = publish(&fixture, "docs", FIRST);
    git(
        &fixture,
        "docs",
        &["checkout", "-q", "-b", "feature/x"],
        LATER,
    );
    edit_search(&fixture, "docs/", "acme/docs#41");
    commit(&fixture, "docs", LATER);
    fixture.gh_search("github.com", "acme/docs", &[]);
    let body = planned_against(&first);
    issue(
        &fixture,
        41,
        GhIssue {
            title: "Recipe search",
            body: &body,
            ..GhIssue::default()
        },
    );

    let from_web = fixture.run_with_gh("web", &["stale"]);
    let from_docs = fixture.run_with_gh("docs", &["stale"]);

    assert_eq!(from_web.stdout, "1 open issue: none stale\n");
    assert_eq!(from_web.code, 0);
    assert!(from_docs.stdout.ends_with("1 open issue: 1 stale\n"));
    assert_eq!(from_docs.code, 1);
}

#[test]
fn fails_when_an_issue_needs_the_default_branch_and_origin_head_is_unset() {
    let (fixture, _) = docs("stale-no-origin-head", "acme/docs#41", "");
    let unset = ["symbolic-ref", "--delete", "refs/remotes/origin/HEAD"];
    git(&fixture, "", &unset, FIRST);
    issue(
        &fixture,
        41,
        GhIssue {
            title: "Recipe search",
            ..GhIssue::default()
        },
    );

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(run.stdout, "");
    assert_eq!(
        run.stderr,
        "error: the docs clone's `origin/HEAD` isn't set, so its default branch is unknown; \
         set it with `git -C . remote set-head origin --auto`\n"
    );
    assert_eq!(run.code, 2);
}

#[test]
fn notes_a_search_for_feature_lines_that_stopped_at_githubs_limit() {
    let (fixture, _) = docs("stale-search-limit", "", "");
    fixture.gh_search_page("github.com", "acme/docs", None, &[], None, 1200);

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(
        run.stderr,
        "note: the search of `acme/docs` for `feature:` lines stopped at 0 of 1200 issues, \
         since GitHub search returns at most 1,000\n"
    );
    assert_eq!(run.stdout, "0 open issues: none stale\n");
}

#[test]
fn fails_without_a_github_block() {
    let fixture = Fixture::new(
        "stale-no-github",
        &[
            ("manifestdocs.yaml", "name: demo\n"),
            ("demo.md", &node("fd-000001", "Demo")),
        ],
    );

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(
        run.stderr,
        "error: `stale` reads issues from GitHub, and manifestdocs.yaml has no `github` block\n"
    );
    assert_eq!(run.code, 2);
    assert_eq!(fixture.gh_calls(), Vec::<String>::new());
}

#[test]
fn prints_stale_and_unchecked_issues_as_json() {
    let (fixture, first) = docs("stale-json", "acme/docs#41, acme/docs#42", "");
    edit_search(&fixture, "", "acme/docs#41, acme/docs#42");
    let later = publish(&fixture, "", LATER);
    let body = planned_against(&first);
    issue(
        &fixture,
        41,
        GhIssue {
            title: "Recipe search",
            body: &body,
            ..GhIssue::default()
        },
    );
    issue(
        &fixture,
        42,
        GhIssue {
            title: "Ranking",
            body: "spec-ref: 0000000\n",
            ..GhIssue::default()
        },
    );

    let run = fixture.run_with_gh("", &["stale", "--json"]);

    let output: Value = serde_json::from_str(&run.stdout).expect("JSON output");
    assert_eq!(
        output,
        json!({
            "schema": 1,
            "command": "stale",
            "data": {
                "open": 2,
                "stale": [{
                    "issue": "acme/docs#41",
                    "title": "Recipe search",
                    "node": "fd-000002",
                    "file": "search/search.md",
                    "spec_ref": first,
                    "created_at": "2026-10-01T09:30:00Z",
                    "since": { "commit": first, "date": "2026-10-01" },
                    "changed_in": { "commit": later, "date": "2026-10-10" },
                    "changes": [{ "path": "search/search.md", "added": 1, "removed": 0 }]
                }],
                "not_checked": [{
                    "issue": "acme/docs#42",
                    "title": "Ranking",
                    "node": "fd-000002",
                    "file": "search/search.md",
                    "error": "its spec-ref `0000000` isn't a commit in the docs clone"
                }]
            }
        })
    );
    assert_eq!(run.code, 1);
}

/// Has the fake `gh` know `acme/docs#41`, Recipe search, planned against `spec_ref`.
fn planned_issue(fixture: &Fixture, spec_ref: &str) {
    let body = planned_against(spec_ref);
    issue(
        fixture,
        41,
        GhIssue {
            title: "Recipe search",
            body: &body,
            ..GhIssue::default()
        },
    );
}

/// A spec-ref on a branch that was squash-merged, or never merged, would look stale forever.
#[test]
fn reports_a_spec_ref_outside_the_history_read_as_not_checked() {
    let (fixture, _) = docs("stale-unmerged-ref", "acme/docs#41", "");
    git(&fixture, "", &["checkout", "-q", "-b", "proposal"], LATER);
    edit_search(&fixture, "", "acme/docs#41");
    let proposal = commit(&fixture, "", LATER);
    git(&fixture, "", &["checkout", "-q", "main"], LATER);
    // Squash-merged: the same change in a new commit.
    edit_search(&fixture, "", "acme/docs#41");
    publish(&fixture, "", "2026-10-11T10:00:00Z");
    let proposal = short(&fixture, "", &proposal);
    planned_issue(&fixture, &proposal);

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(
        run.stdout,
        format!(
            "acme/docs#41 Recipe search\n  \
             Search (fd-000002, draft)  search/search.md\n  \
             can't check it: its spec-ref `{proposal}` isn't in the history of `HEAD`\n\
             \n\
             1 open issue: none stale, 1 not checked\n"
        )
    );
    assert_eq!(run.code, 1);
}

#[test]
fn fails_in_a_shallow_clone() {
    let (fixture, _) = docs("stale-shallow", "acme/docs#41", "");
    edit_search(&fixture, "", "acme/docs#41");
    publish(&fixture, "", LATER);
    let origin = format!("file://{}", fixture.path("").display());
    git(
        &fixture,
        "",
        &["clone", "-q", "--depth", "1", &origin, "shallow"],
        LATER,
    );
    planned_issue(&fixture, "0000000");

    let run = fixture.run_with_gh("shallow", &["stale"]);

    assert_eq!(
        run.stderr,
        "error: the docs clone `.` is shallow, so `stale` can't read its history; fetch all of \
         it, such as with `git -C . fetch --unshallow`, or `fetch-depth: 0` for actions/checkout\n"
    );
    assert_eq!(run.code, 2);
}

/// A partial clone has the commits and folders but only the files it checked out, and the
/// tool never fetches, so it can't count the lines that changed.
#[test]
fn reports_an_issue_whose_diff_a_partial_clone_cant_read_as_not_checked() {
    let (fixture, first) = docs("stale-partial", "acme/docs#41", "");
    edit_search(&fixture, "", "acme/docs#41");
    let later = publish(&fixture, "", LATER);
    git(
        &fixture,
        "",
        &["config", "uploadpack.allowFilter", "true"],
        LATER,
    );
    let origin = format!("file://{}", fixture.path("").display());
    let clone = ["clone", "-q", "--filter=blob:none", &origin, "partial"];
    git(&fixture, "", &clone, LATER);
    planned_issue(&fixture, &first);

    let run = fixture.run_with_gh("partial", &["stale"]);

    let why = format!(
        "  can't check it: the docs clone can't diff {} with {}: ",
        short(&fixture, "", &first),
        short(&fixture, "", &later)
    );
    assert!(run.stdout.contains(&why), "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout
            .ends_with("\n1 open issue: none stale, 1 not checked\n")
    );
    assert_eq!(run.code, 1);
}

/// The folders that were deeper nodes when the issue was planned stay left out, so a child
/// node that moves isn't read as the parent's files deleted.
#[test]
fn leaves_out_a_child_node_that_moved_since_the_baseline() {
    let (fixture, first) = docs("stale-moved-child", "acme/docs#41", "");
    fs::rename(
        fixture.path("search/filters"),
        fixture.path("search/facets"),
    )
    .unwrap();
    let facets = fixture.path("search/facets");
    fs::rename(facets.join("filters.md"), facets.join("facets.md")).unwrap();
    publish(&fixture, "", LATER);
    planned_issue(&fixture, &first);

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(run.stdout, "1 open issue: none stale\n");
    assert_eq!(run.code, 0);
}

/// The root node owns every file in the tree except its child nodes, decision records and
/// the tree's config.
#[test]
fn watches_the_files_the_root_node_owns() {
    let mut files = files("", "", "");
    files[1].1 = feature("fd-000001", "Demo", "acme/docs#41");
    let (fixture, first) = docs_with("stale-root", &files);
    fs::write(
        fixture.path("manifestdocs.yaml"),
        format!("{CONFIG}# Edited\n"),
    )
    .unwrap();
    edit_search(&fixture, "", "");
    fs::create_dir_all(fixture.path("decisions")).unwrap();
    fs::write(fixture.path("decisions/2026-10-10-scope.md"), "# Scope\n").unwrap();
    publish(&fixture, "", LATER);
    planned_issue(&fixture, &first);
    let unchanged = fixture.run_with_gh("", &["stale"]);
    fs::write(fixture.path("glossary.md"), "# Glossary\n").unwrap();
    let later = publish(&fixture, "", "2026-10-12T10:00:00Z");

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(unchanged.stdout, "1 open issue: none stale\n");
    assert_eq!(
        run.stdout,
        format!(
            "acme/docs#41 Recipe search\n  \
             Demo (fd-000001, draft)  demo.md\n  \
             changed in {} (2026-10-12) after its spec-ref {} (2026-10-01)\n    \
             glossary.md (+1 -0)\n\
             \n\
             1 open issue: 1 stale\n",
            short(&fixture, "", &later),
            short(&fixture, "", &first)
        )
    );
}

/// Folder names are matched literally, so `*`, `[` and a leading `:` aren't read as a
/// pattern that matches `rank1` too.
#[test]
fn matches_folder_names_with_pattern_characters_literally() {
    let mut files = files("", "", "");
    let odd = ":ra*k[1]/:ra*k[1].md";
    files.push((odd.into(), feature("fd-00000a", "Rank", "acme/docs#41")));
    files.push(("rank1/rank1.md".into(), node("fd-00000b", "Rank one")));
    let (fixture, first) = docs_with("stale-pattern", &files);
    let edited = format!("{}More.\n", feature("fd-00000a", "Rank", "acme/docs#41"));
    fs::write(fixture.path(odd), edited).unwrap();
    fs::write(
        fixture.path("rank1/rank1.md"),
        node("fd-00000b", "Rank, edited"),
    )
    .unwrap();
    let later = publish(&fixture, "", LATER);
    planned_issue(&fixture, &first);

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(
        run.stdout,
        format!(
            "acme/docs#41 Recipe search\n  \
             Rank (fd-00000a, draft)  {odd}\n  \
             changed in {} (2026-10-10) after its spec-ref {} (2026-10-01)\n    \
             {odd} (+1 -0)\n\
             \n\
             1 open issue: 1 stale\n",
            short(&fixture, "", &later),
            short(&fixture, "", &first)
        )
    );
}

/// A change made on a branch before the issue was created, but merged after, wasn't on the
/// default branch when the issue was planned, so it counts.
#[test]
fn counts_a_branch_merged_after_the_issue_was_created() {
    let (fixture, first) = docs("stale-merged", "acme/docs#41", "");
    git(&fixture, "", &["checkout", "-q", "-b", "side"], LATER);
    edit_search(&fixture, "", "acme/docs#41");
    let side = commit(&fixture, "", "2026-10-03T10:00:00Z");
    git(&fixture, "", &["checkout", "-q", "main"], LATER);
    let merge = ["merge", "-q", "--no-ff", "-m", "Merge side", "side"];
    git(&fixture, "", &merge, LATER);
    let merged = git(&fixture, "", &["rev-parse", "HEAD"], LATER);
    push(&fixture, "", &merged);
    let details = GhIssue {
        title: "Recipe search",
        created_at: Some("2026-10-05T09:00:00Z"),
        ..GhIssue::default()
    };
    issue(&fixture, 41, details);

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(
        run.stdout,
        format!(
            "acme/docs#41 Recipe search\n  \
             Search (fd-000002, draft)  search/search.md\n  \
             changed in {} (2026-10-03) after {} (2026-10-01), the docs when it was created\n    \
             search/search.md (+1 -0)\n\
             \n\
             1 open issue: 1 stale\n",
            short(&fixture, "", &side),
            short(&fixture, "", &first)
        )
    );
}

#[test]
fn prints_an_issue_older_than_the_docs_as_json() {
    let (fixture, first) = docs("stale-json-older", "acme/docs#41", "");
    let details = GhIssue {
        title: "Recipe search",
        created_at: Some("2026-09-01T09:00:00Z"),
        ..GhIssue::default()
    };
    issue(&fixture, 41, details);

    let run = fixture.run_with_gh("", &["stale", "--json"]);

    let output: Value = serde_json::from_str(&run.stdout).expect("JSON output");
    assert_eq!(
        output["data"]["stale"],
        json!([{
            "issue": "acme/docs#41",
            "title": "Recipe search",
            "node": "fd-000002",
            "file": "search/search.md",
            "spec_ref": null,
            "created_at": "2026-09-01T09:00:00Z",
            "since": null,
            "changed_in": { "commit": first, "date": "2026-10-01" },
            "changes": [
                { "path": "search/images/flow.png", "added": null, "removed": null },
                { "path": "search/search.md", "added": 9, "removed": 0 }
            ]
        }])
    );
}

/// Planning sets an issue's spec-ref, then adds the issue to the node's `work`, so a commit
/// that only changes `work` doesn't make the issue stale. A later change to the body does.
#[test]
fn ignores_a_change_that_only_adds_issues_to_the_nodes_work() {
    let (fixture, first) = docs("stale-work-only", "", "");
    let search = fixture.path("search/search.md");
    fs::write(&search, feature("fd-000002", "Search", "acme/docs#41")).unwrap();
    publish(&fixture, "", LATER);
    planned_issue(&fixture, &first);
    let planned = fixture.run_with_gh("", &["stale"]);
    edit_search(&fixture, "", "acme/docs#41");
    let edited = publish(&fixture, "", "2026-10-12T10:00:00Z");

    let run = fixture.run_with_gh("", &["stale"]);

    assert_eq!(planned.stdout, "1 open issue: none stale\n");
    assert_eq!(planned.code, 0);
    assert_eq!(
        run.stdout,
        format!(
            "acme/docs#41 Recipe search\n  \
             Search (fd-000002, draft)  search/search.md\n  \
             changed in {} (2026-10-12) after its spec-ref {} (2026-10-01)\n    \
             search/search.md (+2 -1)\n\
             \n\
             1 open issue: 1 stale\n",
            short(&fixture, "", &edited),
            short(&fixture, "", &first)
        )
    );
}
