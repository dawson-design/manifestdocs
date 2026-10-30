use std::fs;

use serde_json::{Value, json};

use crate::docs_source::{git, workspace};
use crate::support::{Fixture, node};

/// The docs workspace with three records on Search: `dc-000000a` merged into `origin/HEAD`,
/// `dc-000000c` committed only on the checked-out `feature/x`, and `dc-000000b` uncommitted.
fn records_in_versions(name: &str) -> Fixture {
    let (fixture, _, _) = workspace(name);
    let write = |file: &str, id: &str, title: &str, date: &str| {
        let text = record(id, "fd-7c2e41", title, "accepted", date);
        let path = fixture.path(&format!("work/docs/search/decisions/{file}"));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    };
    let docs = "work/docs";
    write("2026-10-01-merged.md", "dc-000000a", "Merged", "2026-10-01");
    git(&fixture, docs, &["add", "-A"]);
    git(
        &fixture,
        docs,
        &["commit", "-q", "-m", "Record a merged decision"],
    );
    git(&fixture, docs, &["push", "-q", "origin", "feature/x:main"]);
    git(&fixture, docs, &["fetch", "-q", "origin"]);
    write(
        "2026-10-03-branch.md",
        "dc-000000c",
        "On the branch",
        "2026-10-03",
    );
    git(&fixture, docs, &["add", "-A"]);
    git(
        &fixture,
        docs,
        &["commit", "-q", "-m", "Record a branch decision"],
    );
    write(
        "2026-10-02-draft.md",
        "dc-000000b",
        "Uncommitted",
        "2026-10-02",
    );
    fixture
}

#[test]
fn reads_origin_head_through_a_pointer_not_uncommitted_records() {
    let fixture = records_in_versions("decisions-origin-head");

    let run = fixture.run("work/web", &["decisions"]);

    assert_eq!(
        run.stdout,
        "Search, edited (fd-7c2e41, draft)\n\
         \x20 dc-000000a  accepted  2026-10-01  Merged  ../docs/search/decisions/2026-10-01-merged.md\n"
    );
    assert_eq!(run.code, 0, "{}", run.stderr);
}

#[test]
fn ref_reads_the_records_on_another_branch() {
    let fixture = records_in_versions("decisions-ref");

    let run = fixture.run(
        "work/web",
        &["decisions", "fd-7c2e41", "--ref", "feature/x"],
    );

    assert_eq!(
        run.stdout,
        "Search, edited (fd-7c2e41, draft)\n\
         \x20 dc-000000c  accepted  2026-10-03  On the branch  ../docs/search/decisions/2026-10-03-branch.md\n\
         \x20 dc-000000a  accepted  2026-10-01  Merged  ../docs/search/decisions/2026-10-01-merged.md\n"
    );
    assert_eq!(run.code, 0, "{}", run.stderr);
}

#[test]
fn fails_with_the_set_head_command_when_origin_head_is_unset() {
    let fixture = records_in_versions("decisions-no-origin-head");
    let unset = ["remote", "set-head", "origin", "-d"];
    git(&fixture, "work/docs", &unset);

    let run = fixture.run("work/web", &["decisions"]);

    assert_eq!(
        run.stderr,
        "error: the docs clone's `origin/HEAD` isn't set, so its default branch is unknown; \
         set it with `git -C ../docs remote set-head origin --auto`\n"
    );
    assert_eq!(run.code, 2);
}

/// The tree is in the code repo, reached through the repo's own pointer, so the working
/// tree is the docs clone and an uncommitted record is listed though `origin/HEAD` is set.
#[test]
fn reads_the_working_tree_when_the_tree_is_in_the_working_repo() {
    let draft = record(
        "dc-000000b",
        "fd-7c2e41",
        "Uncommitted",
        "accepted",
        "2026-10-02",
    );
    let fixture = Fixture::new(
        "decisions-single-repo",
        &[
            (".manifestdocs/config.yaml", "tree: docs\n"),
            ("src/main.rs", "fn main() {}\n"),
            ("docs/manifestdocs.yaml", "name: cookbook\n"),
            ("docs/cookbook.md", &node("fd-0a0001", "Cookbook")),
            ("docs/search/search.md", &node("fd-7c2e41", "Search")),
        ],
    );
    fs::remove_dir_all(fixture.path(".git")).unwrap();
    git(&fixture, "", &["init", "-q", "-b", "main"]);
    git(&fixture, "", &["add", "-A"]);
    git(&fixture, "", &["commit", "-q", "-m", "Add the tree"]);
    git(
        &fixture,
        "",
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    );
    let origin_head = [
        "symbolic-ref",
        "refs/remotes/origin/HEAD",
        "refs/remotes/origin/main",
    ];
    git(&fixture, "", &origin_head);
    let file = "docs/search/decisions/2026-10-02-draft.md";
    fs::create_dir_all(fixture.path("docs/search/decisions")).unwrap();
    fs::write(fixture.path(file), draft).unwrap();

    let run = fixture.run("src", &["decisions", "fd-7c2e41"]);

    assert_eq!(
        run.stdout,
        "Search (fd-7c2e41, draft)\n\
         \x20 dc-000000b  accepted  2026-10-02  Uncommitted  ../docs/search/decisions/2026-10-02-draft.md\n"
    );
    assert_eq!(run.code, 0, "{}", run.stderr);
}

fn record(id: &str, node: &str, title: &str, status: &str, date: &str) -> String {
    format!(
        "---\nid: {id}\nnode: {node}\ntitle: \"{title}\"\nstatus: {status}\ntopic: t-{id}\ndate: {date}\n---\n\n**Decision.** {title}.\n"
    )
}

/// A root node, Import and Search below it, and records on the root, Import and Search,
/// one of them with frontmatter errors.
fn cookbook(name: &str) -> Fixture {
    let root = node("fd-000001", "Demo");
    let import = node("fd-000003", "Import");
    let search = node("fd-000002", "Search");
    let github = record(
        "dc-000000a",
        "fd-000001",
        "No GitHub writes",
        "accepted",
        "2026-10-01",
    );
    let engine = record(
        "dc-000000b",
        "fd-000002",
        "Use SQLite FTS",
        "rejected",
        "2026-10-02",
    );
    let page = record(
        "dc-000000c",
        "fd-000002",
        "Show 20 per page",
        "accepted",
        "2026-10-28",
    );
    let csv = record(
        "dc-000000d",
        "fd-000003",
        "Import CSV first",
        "proposed",
        "2026-10-03",
    );
    let broken = "---\nid: dc-000000e\nnode: fd-000003\nstatus: accepted\n---\n";
    Fixture::new(
        name,
        &[
            ("manifestdocs.yaml", "name: demo\n"),
            ("demo.md", &root),
            ("decisions/2026-10-01-github.md", &github),
            ("import/import.md", &import),
            ("import/decisions/2026-10-03-csv.md", &csv),
            ("import/decisions/2026-10-04-broken.md", broken),
            ("search/search.md", &search),
            ("search/decisions/2026-10-02-engine.md", &engine),
            ("search/decisions/2026-10-28-page-size.md", &page),
        ],
    )
}

#[test]
fn lists_a_nodes_records_when_given_its_folder() {
    let fixture = cookbook("decisions-path");

    let run = fixture.run("", &["decisions", "search"]);

    assert_eq!(
        run.stdout,
        "Search (fd-000002, draft)\n\
         \x20 dc-000000c  accepted  2026-10-28  Show 20 per page  search/decisions/2026-10-28-page-size.md\n\
         \x20 dc-000000b  rejected  2026-10-02  Use SQLite FTS  search/decisions/2026-10-02-engine.md\n\
         \n\
         Demo (fd-000001, draft)\n\
         \x20 dc-000000a  accepted  2026-10-01  No GitHub writes  decisions/2026-10-01-github.md\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn json_lists_every_record_in_the_tree_in_outline_order() {
    let fixture = cookbook("decisions-json-all");

    let run = fixture.run("", &["decisions", "--json"]);

    let parsed: Value = serde_json::from_str(&run.stdout).expect("one JSON object");
    let records = parsed["data"]["records"].as_array().expect("a list");
    let ids: Vec<&str> = records.iter().filter_map(|r| r["id"].as_str()).collect();
    let nodes: Vec<&str> = records.iter().filter_map(|r| r["node"].as_str()).collect();
    assert_eq!(
        ids,
        [
            "dc-000000a",
            "dc-000000d",
            "dc-000000e",
            "dc-000000c",
            "dc-000000b"
        ]
    );
    assert_eq!(
        nodes,
        [
            "fd-000001",
            "fd-000003",
            "fd-000003",
            "fd-000002",
            "fd-000002"
        ]
    );
    assert_eq!(parsed["command"], "decisions");
    assert_eq!(run.code, 0);
}

#[test]
fn lists_every_record_grouped_by_node_newest_first() {
    let fixture = cookbook("decisions-all");

    let run = fixture.run("", &["decisions"]);

    assert_eq!(
        run.stdout,
        "Demo (fd-000001, draft)\n\
         \x20 dc-000000a  accepted  2026-10-01  No GitHub writes  decisions/2026-10-01-github.md\n\
         \n\
         Import (fd-000003, draft)\n\
         \x20 dc-000000d  proposed  2026-10-03  Import CSV first  import/decisions/2026-10-03-csv.md\n\
         \x20 dc-000000e  has problems  import/decisions/2026-10-04-broken.md\n\
         \n\
         Search (fd-000002, draft)\n\
         \x20 dc-000000c  accepted  2026-10-28  Show 20 per page  search/decisions/2026-10-28-page-size.md\n\
         \x20 dc-000000b  rejected  2026-10-02  Use SQLite FTS  search/decisions/2026-10-02-engine.md\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn lists_a_nodes_records_then_its_ancestors() {
    let fixture = cookbook("decisions-node");

    let run = fixture.run("search", &["decisions", "fd-000002"]);

    assert_eq!(
        run.stdout,
        "Search (fd-000002, draft)\n\
         \x20 dc-000000c  accepted  2026-10-28  Show 20 per page  decisions/2026-10-28-page-size.md\n\
         \x20 dc-000000b  rejected  2026-10-02  Use SQLite FTS  decisions/2026-10-02-engine.md\n\
         \n\
         Demo (fd-000001, draft)\n\
         \x20 dc-000000a  accepted  2026-10-01  No GitHub writes  ../decisions/2026-10-01-github.md\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn says_when_there_are_no_records() {
    let root = node("fd-000001", "Demo");
    let fixture = Fixture::new(
        "decisions-none",
        &[("manifestdocs.yaml", "name: demo\n"), ("demo.md", &root)],
    );

    let run = fixture.run("", &["decisions"]);

    assert_eq!(run.stdout, "no decision records\n");
    assert_eq!(run.code, 0);
}

#[test]
fn json_lists_each_record_with_its_node() {
    let fixture = cookbook("decisions-json");

    let run = fixture.run("", &["decisions", "fd-000003", "--json"]);

    let parsed: Value = serde_json::from_str(&run.stdout).expect("one JSON object");
    assert_eq!(
        parsed,
        json!({
            "schema": 1,
            "command": "decisions",
            "data": {
                "records": [
                    {
                        "id": "dc-000000d",
                        "node": "fd-000003",
                        "title": "Import CSV first",
                        "status": "proposed",
                        "topic": "t-dc-000000d",
                        "date": "2026-10-03",
                        "supersedes": null,
                        "file": "import/decisions/2026-10-03-csv.md"
                    },
                    {
                        "id": "dc-000000e",
                        "node": "fd-000003",
                        "title": null,
                        "status": null,
                        "topic": null,
                        "date": null,
                        "supersedes": null,
                        "file": "import/decisions/2026-10-04-broken.md"
                    },
                    {
                        "id": "dc-000000a",
                        "node": "fd-000001",
                        "title": "No GitHub writes",
                        "status": "accepted",
                        "topic": "t-dc-000000a",
                        "date": "2026-10-01",
                        "supersedes": null,
                        "file": "decisions/2026-10-01-github.md"
                    }
                ]
            }
        })
    );
    assert_eq!(run.code, 0);
}
