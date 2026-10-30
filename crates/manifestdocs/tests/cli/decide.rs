use std::fs;
use std::process::Command;

use serde_json::{Value, json};

use crate::docs_source::{set_read_only, workspace};
use crate::support::{Fixture, Run, node};

const DATE: (&str, &str) = ("MANIFESTDOCS_TEST_DATE", "2026-10-29");
const WRITTEN: &str = "search/decisions/2026-10-29-page-size.md";

/// A record as an author writes it, for the Search node, with `topic` on line 4.
const DRAFT: &str = "---\nnode: fd-000002\ntitle: \"Search shows 20 recipes per page\"\ntopic: page-size\ndecided_by: octocat\nsource: [example-org/cookbook-web#212]\n---\n\n**Decision.** Search shows 20 recipes per page.\n\n**Why.** Readers rarely scroll past 20 results.\n";

/// A tree with a root node and a Search node, with `config` as its manifestdocs.yaml and
/// `files` added, such as a draft record.
fn cookbook(name: &str, config: &str, files: &[(&str, &str)]) -> Fixture {
    let root = node("fd-000001", "Demo");
    let search = node("fd-000002", "Search");
    let mut all = vec![
        ("manifestdocs.yaml", config),
        ("demo.md", root.as_str()),
        ("search/search.md", search.as_str()),
    ];
    all.extend_from_slice(files);
    Fixture::new(name, &all)
}

/// Runs `decide` from the fixture root with `args`, on the pinned date.
fn decide(fixture: &Fixture, args: &[&str]) -> Run {
    let args = [&["decide"], args].concat();
    fixture.run_with("", &args, "", &[DATE])
}

/// The record `decide` writes for `DRAFT`, given its `id` and `status`.
fn written(id: &str, status: &str) -> String {
    format!(
        "---\nid: {id}\nnode: fd-000002\ntitle: \"Search shows 20 recipes per page\"\nstatus: {status}\ntopic: page-size\ndecided_by: \"octocat\"\ndate: 2026-10-29\nsource: [example-org/cookbook-web#212]\n---\n\n**Decision.** Search shows 20 recipes per page.\n\n**Why.** Readers rarely scroll past 20 results.\n"
    )
}

/// The decision ID on the first `id:` line in `text`.
fn id_in(text: &str) -> String {
    let line = text.lines().find(|line| line.starts_with("id: "));
    let id = line.expect("an id line").trim_start_matches("id: ");
    let hex = id.strip_prefix("dc-").expect("a decision ID");
    assert!(
        hex.len() == 7
            && hex
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
        "{id}"
    );
    id.to_owned()
}

fn parsed(run: &Run) -> Value {
    serde_json::from_str(&run.stdout).expect("stdout is one JSON object")
}

#[test]
fn writes_the_record_into_its_nodes_decisions_folder_and_prints_its_path() {
    let fixture = cookbook("decide-file", "name: demo\n", &[("draft.md", DRAFT)]);

    let run = decide(&fixture, &["--file", "draft.md"]);

    let text = fixture.read(WRITTEN);
    assert_eq!(run.stdout, format!("{WRITTEN}\n"));
    assert_eq!(run.stderr, "");
    assert_eq!(run.code, 0);
    assert_eq!(text, written(&id_in(&text), "accepted"));
    assert_eq!(fixture.check("").stdout, "2 nodes: no problems\n");
}

#[test]
fn reads_the_record_from_stdin() {
    let fixture = cookbook("decide-stdin", "name: demo\n", &[]);

    let run = fixture.run_with("", &["decide", "--file", "-"], DRAFT, &[DATE]);

    let text = fixture.read(WRITTEN);
    assert_eq!(run.stdout, format!("{WRITTEN}\n"));
    assert_eq!(run.code, 0);
    assert_eq!(text, written(&id_in(&text), "accepted"));
}

#[test]
fn reports_every_input_error_in_one_pass_with_its_line_and_fix() {
    let draft = "---\nid: dc-1234567\nnode: fd-000002\ntitle: Search page size\nstatus: accepted\ntopic: page-size\nowner: search-team\n---\n";
    let fixture = cookbook(
        "decide-errors",
        "name: demo\ndecisions:\n  merge: proposed\n",
        &[("draft.md", draft)],
    );

    let run = decide(&fixture, &["--file", "draft.md"]);

    assert_eq!(
        run.stdout,
        "draft.md:2: error: `id` is given, but decide generates it\n  fix: remove `id`\n\
         draft.md:4: error: `title` must be double-quoted\n  fix: put the value of `title` in double quotes\n\
         draft.md:5: error: status `accepted` isn't allowed: decide writes `proposed` under `decisions.merge: proposed`\n  fix: remove `status`, and decide sets it\n\
         draft.md:7: error: unknown field `owner`\n  fix: remove `owner`, or correct its name\n\
         \n4 errors: nothing written\n"
    );
    assert_eq!(run.code, 1);
    assert!(!fixture.path("search/decisions").exists());
}

#[test]
fn rejects_a_node_that_isnt_in_the_tree() {
    let draft = DRAFT.replace("fd-000002", "fd-00000f");
    let fixture = cookbook("decide-no-node", "name: demo\n", &[("draft.md", &draft)]);

    let run = decide(&fixture, &["--file", "draft.md"]);

    assert_eq!(
        run.stdout,
        "draft.md:2: error: `node` names `fd-00000f`, which isn't a node in this tree\n  fix: set `node` to a node ID from `mfd tree`\n\
         \n1 error: nothing written\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn rejects_input_without_frontmatter() {
    let fixture = cookbook(
        "decide-no-frontmatter",
        "name: demo\n",
        &[("draft.md", "**Decision.** Use 20.\n")],
    );

    let run = decide(&fixture, &["--file", "draft.md"]);

    assert_eq!(
        run.stdout,
        "draft.md:1: error: the record has no frontmatter\n  fix: start it with its fields between `---` lines, as a record file does\n\
         \n1 error: nothing written\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn dry_run_prints_the_record_and_its_path_without_writing() {
    let fixture = cookbook("decide-dry-run", "name: demo\n", &[("draft.md", DRAFT)]);

    let run = decide(&fixture, &["--file", "draft.md", "--dry-run"]);

    let id = id_in(&run.stdout);
    assert_eq!(
        run.stdout,
        format!("{WRITTEN}\n\n{}", written(&id, "accepted"))
    );
    assert_eq!(run.code, 0);
    assert!(!fixture.path("search/decisions").exists());
}

#[test]
fn writes_proposed_when_the_tree_merges_decisions_as_proposed() {
    let fixture = cookbook(
        "decide-merge-proposed",
        "name: demo\ndecisions:\n  merge: proposed\n",
        &[("draft.md", DRAFT)],
    );

    let run = decide(&fixture, &["--file", "draft.md"]);

    let text = fixture.read(WRITTEN);
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(text, written(&id_in(&text), "proposed"));
}

#[test]
fn keeps_a_supplied_proposed_status_under_review() {
    let draft = DRAFT.replace("topic:", "status: proposed\ntopic:");
    let fixture = cookbook(
        "decide-keeps-proposed",
        "name: demo\n",
        &[("draft.md", &draft)],
    );

    let run = decide(&fixture, &["--file", "draft.md"]);

    let text = fixture.read(WRITTEN);
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(text, written(&id_in(&text), "proposed"));
}

#[test]
fn unattended_writes_proposed_to_the_repos_local_drafts_folder_and_ignores_it() {
    let fixture = cookbook("decide-unattended", "name: demo\n", &[("draft.md", DRAFT)]);

    let args = ["decide", "--file", "../draft.md", "--unattended"];
    let run = fixture.run_with("search", &args, "", &[DATE]);

    let path = ".manifestdocs/decisions/2026-10-29-page-size.md";
    let text = fixture.read(path);
    assert_eq!(run.stdout, format!("../{path}\n"));
    assert_eq!(run.code, 0);
    assert_eq!(text, written(&id_in(&text), "proposed"));
    assert_eq!(fixture.read(".manifestdocs/decisions/.gitignore"), "*\n");
    assert!(!fixture.path("search/decisions").exists());
}

#[test]
fn out_writes_to_the_given_folder_without_a_gitignore() {
    let fixture = cookbook("decide-out", "name: demo\n", &[("draft.md", DRAFT)]);

    let run = decide(&fixture, &["--file", "draft.md", "--out", "drafts"]);

    let path = "drafts/2026-10-29-page-size.md";
    let text = fixture.read(path);
    assert_eq!(run.stdout, format!("{path}\n"));
    assert_eq!(run.code, 0);
    assert_eq!(text, written(&id_in(&text), "accepted"));
    assert!(!fixture.path("drafts/.gitignore").exists());
    assert!(!fixture.path("search/decisions").exists());
}

#[test]
fn a_rerun_with_the_same_node_topic_and_body_prints_the_existing_path() {
    let fixture = cookbook("decide-rerun", "name: demo\n", &[("draft.md", DRAFT)]);
    let first = decide(&fixture, &["--file", "draft.md"]);
    let retitled = DRAFT.replace("title: \"Search shows", "title: \"Show");
    fs::write(fixture.path("draft.md"), retitled).unwrap();

    let second = fixture.run_with(
        "",
        &["decide", "--file", "draft.md"],
        "",
        &[("MANIFESTDOCS_TEST_DATE", "2026-11-02")],
    );

    let files = fs::read_dir(fixture.path("search/decisions")).unwrap();
    assert_eq!(first.stdout, format!("{WRITTEN}\n"));
    assert_eq!(second.stdout, first.stdout);
    assert_eq!(second.code, 0);
    assert_eq!(files.count(), 1);
}

#[test]
fn a_different_body_on_a_recorded_topic_fails_and_suggests_supersedes() {
    let fixture = cookbook(
        "decide-different-body",
        "name: demo\n",
        &[("draft.md", DRAFT)],
    );
    decide(&fixture, &["--file", "draft.md"]);
    let id = id_in(&fixture.read(WRITTEN));
    let changed = DRAFT.replace("Readers rarely", "People rarely");
    fs::write(fixture.path("draft.md"), changed).unwrap();

    let run = decide(&fixture, &["--file", "draft.md"]);

    assert_eq!(
        run.stdout,
        format!(
            "draft.md:4: error: topic `page-size` already has a different decision, `{id}`\n  fix: add `supersedes: {id}` to replace it, or keep its body to rerun it\n\
             \n1 error: nothing written\n"
        )
    );
    assert_eq!(run.code, 1);
}

#[test]
fn writes_a_record_that_supersedes_the_one_on_its_topic_and_marks_that_one() {
    let fixture = superseding("decide-supersedes", OLDER_FILE, "2026-10-01");

    let run = decide(&fixture, &["--file", "draft.md"]);

    let text = fixture.read(WRITTEN);
    assert_eq!(run.stdout, format!("{WRITTEN}\nsuperseded: {OLDER_FILE}\n"));
    assert_eq!(run.code, 0);
    assert_eq!(text, superseding_record(&id_in(&text)));
    assert_eq!(fixture.read(OLDER_FILE), older("2026-10-01", "superseded"));
    assert_eq!(fixture.check("").stdout, "2 nodes: no problems\n");
}

#[test]
fn dry_run_shows_the_record_and_the_superseded_change_without_writing() {
    let fixture = superseding("decide-supersedes-dry-run", OLDER_FILE, "2026-10-01");

    let run = decide(&fixture, &["--file", "draft.md", "--dry-run"]);

    let record = superseding_record(&id_in(&run.stdout));
    let marked = older("2026-10-01", "superseded");
    assert_eq!(
        run.stdout,
        format!("{WRITTEN}\n\n{record}\nsuperseded: {OLDER_FILE}\n\n{marked}")
    );
    assert_eq!(run.code, 0);
    assert!(!fixture.path(WRITTEN).exists());
    assert_eq!(fixture.read(OLDER_FILE), older("2026-10-01", "accepted"));
}

#[test]
fn superseding_a_record_from_the_same_day_takes_the_next_free_name() {
    let fixture = superseding("decide-same-day", WRITTEN, "2026-10-29");
    let next = "search/decisions/2026-10-29-page-size-2.md";

    let dry_run = decide(&fixture, &["--file", "draft.md", "--dry-run"]);
    let run = decide(&fixture, &["--file", "draft.md"]);

    assert!(
        dry_run.stdout.starts_with(&format!("{next}\n\n")),
        "{}",
        dry_run.stdout
    );
    assert_eq!(run.stdout, format!("{next}\nsuperseded: {WRITTEN}\n"));
    assert_eq!(run.code, 0);
    assert_eq!(fixture.read(WRITTEN), older("2026-10-29", "superseded"));
    assert!(fixture.path(next).exists());
}

#[test]
fn json_reports_the_record_it_supersedes() {
    let fixture = superseding("decide-supersedes-json", OLDER_FILE, "2026-10-01");

    let run = decide(&fixture, &["--file", "draft.md", "--json"]);

    let data = &parsed(&run)["data"];
    assert_eq!(data["file"], WRITTEN);
    assert_eq!(
        data["superseded"],
        json!({
            "id": "dc-0000001",
            "file": OLDER_FILE,
            "record": older("2026-10-01", "superseded")
        })
    );
    assert_eq!(run.code, 0);
}

#[test]
fn rejects_a_supersedes_that_names_no_record() {
    let draft = DRAFT.replace("topic:", "supersedes: dc-00000ff\ntopic:");
    let fixture = cookbook("decide-no-older", "name: demo\n", &[("draft.md", &draft)]);

    let run = decide(&fixture, &["--file", "draft.md"]);

    assert_eq!(
        run.stdout,
        "draft.md:4: error: `supersedes` names `dc-00000ff`, which isn't a decision record in this tree\n  fix: set `supersedes` to a decision ID from `mfd decisions`\n\
         \n1 error: nothing written\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn writes_lf_from_crlf_input_and_matches_it_on_a_rerun() {
    let crlf = DRAFT.replace('\n', "\r\n");
    let fixture = cookbook("decide-crlf", "name: demo\n", &[("draft.md", &crlf)]);
    let first = decide(&fixture, &["--file", "draft.md"]);
    let text = fixture.read(WRITTEN);

    let rerun = fixture.run_with("", &["decide", "--file", "-"], DRAFT, &[DATE]);

    assert_eq!(first.stdout, format!("{WRITTEN}\n"));
    assert_eq!(text, written(&id_in(&text), "accepted"));
    assert_eq!(rerun.stdout, first.stdout);
    assert_eq!(rerun.code, 0);
}

#[test]
fn reads_input_that_starts_with_a_byte_order_mark() {
    let draft = format!("\u{feff}{DRAFT}");
    let fixture = cookbook("decide-bom", "name: demo\n", &[("draft.md", &draft)]);

    let run = decide(&fixture, &["--file", "draft.md"]);

    let text = fixture.read(WRITTEN);
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(text, written(&id_in(&text), "accepted"));
}

#[test]
fn rejects_frontmatter_without_a_closing_line() {
    let draft = "---\nnode: fd-000002\ntitle: \"Page size\"\ntopic: page-size\n";
    let fixture = cookbook("decide-unclosed", "name: demo\n", &[("draft.md", draft)]);

    let run = decide(&fixture, &["--file", "draft.md"]);

    assert_eq!(
        run.stdout,
        "draft.md:1: error: the frontmatter has no closing `---` line\n  fix: add a `---` line after the last field\n\
         \n1 error: nothing written\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn reports_a_yaml_error_at_the_authors_own_line() {
    let draft = DRAFT.replace("#212]", "#212");
    let fixture = cookbook("decide-yaml-error", "name: demo\n", &[("draft.md", &draft)]);

    let run = decide(&fixture, &["--file", "draft.md"]);

    let first = run.stdout.lines().next().unwrap_or_default();
    assert!(
        first.starts_with("draft.md:6: error: frontmatter isn't valid YAML"),
        "{}",
        run.stdout
    );
    assert_eq!(run.code, 1);
}

#[test]
fn a_rerun_matches_a_record_in_the_out_folder() {
    let fixture = cookbook("decide-out-rerun", "name: demo\n", &[("draft.md", DRAFT)]);
    let first = decide(&fixture, &["--file", "draft.md", "--out", "drafts"]);

    let second = decide(&fixture, &["--file", "draft.md", "--out", "drafts"]);

    let files = fs::read_dir(fixture.path("drafts")).unwrap();
    assert_eq!(first.stdout, "drafts/2026-10-29-page-size.md\n");
    assert_eq!(second.stdout, first.stdout);
    assert_eq!(second.code, 0);
    assert_eq!(files.count(), 1);
}

#[test]
fn unattended_adds_no_gitignore_to_a_drafts_folder_that_already_exists() {
    let fixture = cookbook(
        "decide-unattended-existing",
        "name: demo\n",
        &[
            ("draft.md", DRAFT),
            (".manifestdocs/decisions/notes.txt", "Earlier drafts.\n"),
        ],
    );

    let run = decide(&fixture, &["--file", "draft.md", "--unattended"]);

    assert_eq!(
        run.stdout,
        ".manifestdocs/decisions/2026-10-29-page-size.md\n"
    );
    assert_eq!(run.code, 0);
    assert!(!fixture.path(".manifestdocs/decisions/.gitignore").exists());
}

const OLDER_FILE: &str = "search/decisions/2026-10-01-page-size.md";

/// The record that `DRAFT` with `supersedes: dc-0000001` replaces, dated `date`.
fn older(date: &str, status: &str) -> String {
    format!(
        "---\nid: dc-0000001\nnode: fd-000002\ntitle: \"Search shows 10 recipes per page\"\nstatus: {status}\ntopic: page-size\ndate: {date}\n---\n\n**Decision.** Search shows 10 recipes per page.\n"
    )
}

/// A cookbook with the accepted older record at `file`, dated `date`, and a draft that
/// supersedes it.
fn superseding(name: &str, file: &str, date: &str) -> Fixture {
    let draft = DRAFT.replace("topic:", "supersedes: dc-0000001\ntopic:");
    let older = older(date, "accepted");
    cookbook(
        name,
        "name: demo\n",
        &[(file, &older), ("draft.md", &draft)],
    )
}

/// The record `decide` writes for the superseding draft.
fn superseding_record(id: &str) -> String {
    let source = "source: [example-org/cookbook-web#212]\n";
    let with = format!("{source}supersedes: dc-0000001\n");
    written(id, "accepted").replace(source, &with)
}

/// Runs `git` in the fixture's repo, with its own home folder so no user config applies.
fn git(fixture: &Fixture, args: &[&str]) {
    let output = Command::new("git")
        .args(["-c", "user.name=Sam", "-c", "user.email=sam@example.com"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(fixture.path(""))
        .env("HOME", fixture.path(""))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git runs");
    assert!(output.status.success(), "git {args:?}: {output:?}");
}

/// The cookbook in a git repo on `main`, with `origin/HEAD` pointing at its one commit, and
/// then on a new branch `feature/filters`.
fn on_fresh_branch(name: &str) -> Fixture {
    let fixture = cookbook(name, "name: demo\n", &[("draft.md", DRAFT)]);
    git(&fixture, &["init", "-q", "-b", "main"]);
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "Add the tree"]);
    git(
        &fixture,
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    );
    let origin_head = [
        "symbolic-ref",
        "refs/remotes/origin/HEAD",
        "refs/remotes/origin/main",
    ];
    git(&fixture, &origin_head);
    git(&fixture, &["checkout", "-q", "-b", "feature/filters"]);
    fixture
}

#[test]
fn a_fresh_branch_from_the_default_branch_gets_no_warning() {
    let fixture = on_fresh_branch("decide-fresh-branch");

    let run = decide(&fixture, &["--file", "draft.md"]);

    assert_eq!(run.stderr, "");
    assert_eq!(run.stdout, format!("{WRITTEN}\n"));
    assert_eq!(run.code, 0);
}

#[test]
fn warns_when_the_branch_has_commits_that_arent_on_the_default_branch() {
    let fixture = on_fresh_branch("decide-diverged-branch");
    fs::write(fixture.path("demo.md"), node("fd-000001", "Demo, edited")).unwrap();
    git(&fixture, &["commit", "-q", "-am", "Edit the root"]);

    let run = decide(&fixture, &["--file", "draft.md"]);

    assert_eq!(
        run.stderr,
        "warning: the docs clone is on `feature/filters`, which has commits that aren't on its default branch `main`\n"
    );
    assert_eq!(run.stdout, format!("{WRITTEN}\n"));
    assert_eq!(run.code, 0);
}

#[test]
fn warns_when_the_docs_clone_has_uncommitted_changes() {
    let fixture = on_fresh_branch("decide-uncommitted");
    fs::write(fixture.path("demo.md"), node("fd-000001", "Demo, edited")).unwrap();

    let run = decide(&fixture, &["--file", "draft.md"]);

    assert_eq!(
        run.stderr,
        "warning: the docs clone has uncommitted changes\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn json_prints_the_written_record() {
    let fixture = cookbook("decide-json", "name: demo\n", &[("draft.md", DRAFT)]);

    let run = decide(&fixture, &["--file", "draft.md", "--json"]);

    let text = fixture.read(WRITTEN);
    let id = id_in(&text);
    assert_eq!(
        parsed(&run),
        json!({
            "schema": 1,
            "command": "decide",
            "data": {
                "outcome": "written",
                "id": id,
                "status": "accepted",
                "file": WRITTEN,
                "record": written(&id, "accepted"),
                "superseded": null,
                "errors": [],
                "warnings": []
            }
        })
    );
    assert_eq!(run.code, 0);
}

#[test]
fn json_lists_the_input_errors() {
    let draft = DRAFT.replace("\"Search shows 20 recipes per page\"", "Search page size");
    let fixture = cookbook(
        "decide-json-errors",
        "name: demo\n",
        &[("draft.md", &draft)],
    );

    let run = decide(&fixture, &["--file", "draft.md", "--json"]);

    assert_eq!(
        parsed(&run),
        json!({
            "schema": 1,
            "command": "decide",
            "data": {
                "outcome": "invalid",
                "id": null,
                "status": null,
                "file": null,
                "record": null,
                "superseded": null,
                "errors": [{
                    "file": "draft.md",
                    "line": 3,
                    "field": "title",
                    "severity": "error",
                    "code": "unquoted_field",
                    "message": "`title` must be double-quoted",
                    "fix": "put the value of `title` in double quotes"
                }],
                "warnings": []
            }
        })
    );
    assert_eq!(run.code, 1);
}

#[test]
fn json_names_no_file_for_errors_in_stdin() {
    let draft = DRAFT.replace("\"Search shows 20 recipes per page\"", "Search page size");
    let fixture = cookbook("decide-json-stdin", "name: demo\n", &[]);

    let run = fixture.run_with("", &["decide", "--file", "-", "--json"], &draft, &[DATE]);

    let errors = &parsed(&run)["data"]["errors"];
    assert_eq!(errors[0]["file"], Value::Null);
    assert_eq!(errors[0]["line"], 3);
    assert_eq!(run.code, 1);
}

#[test]
fn json_dry_run_lists_the_clone_warnings() {
    let fixture = on_fresh_branch("decide-json-dry-run");
    fs::write(fixture.path("demo.md"), node("fd-000001", "Demo, edited")).unwrap();
    git(&fixture, &["commit", "-q", "-am", "Edit the root"]);
    fs::write(
        fixture.path("search/search.md"),
        node("fd-000002", "Search, edited"),
    )
    .unwrap();

    let run = decide(&fixture, &["--file", "draft.md", "--dry-run", "--json"]);

    let data = &parsed(&run)["data"];
    let id = data["id"].as_str().unwrap_or_default();
    assert_eq!(data["outcome"], "dry_run");
    assert_eq!(data["file"], WRITTEN);
    assert_eq!(data["record"], written(id, "accepted"));
    assert_eq!(
        data["warnings"],
        json!([
            {
                "code": "ahead_of_default_branch",
                "message": "the docs clone is on `feature/filters`, which has commits that aren't on its default branch `main`"
            },
            {
                "code": "uncommitted_changes",
                "message": "the docs clone has uncommitted changes"
            }
        ])
    );
    assert!(!fixture.path(WRITTEN).exists());
    assert_eq!(run.code, 0);
}

#[test]
fn json_marks_a_rerun_as_unchanged() {
    let fixture = cookbook("decide-json-rerun", "name: demo\n", &[("draft.md", DRAFT)]);
    decide(&fixture, &["--file", "draft.md"]);
    let id = id_in(&fixture.read(WRITTEN));

    let run = decide(&fixture, &["--file", "draft.md", "--json"]);

    let data = &parsed(&run)["data"];
    assert_eq!(data["outcome"], "unchanged");
    assert_eq!(data["id"], id.as_str());
    assert_eq!(data["file"], WRITTEN);
    assert_eq!(data["record"], Value::Null);
    assert_eq!(run.code, 0);
}

#[test]
fn warns_when_a_detached_head_has_commits_that_arent_on_the_default_branch() {
    let fixture = on_fresh_branch("decide-detached-ahead");
    fs::write(fixture.path("demo.md"), node("fd-000001", "Demo, edited")).unwrap();
    git(&fixture, &["commit", "-q", "-am", "Edit the root"]);
    git(&fixture, &["checkout", "-q", "--detach"]);

    let run = decide(&fixture, &["--file", "draft.md"]);

    assert_eq!(
        run.stderr,
        "warning: the docs clone's detached HEAD has commits that aren't on its default branch `main`\n"
    );
    assert_eq!(run.stdout, format!("{WRITTEN}\n"));
    assert_eq!(run.code, 0);
}

const BESIDE_OLDER_FILE: &str = "work/docs/search/decisions/2026-10-01-page-size.md";

/// A code repo beside a docs clone whose Search node has the older record, and a draft
/// that supersedes it, for runs from the code repo.
fn beside_the_docs(name: &str) -> (Fixture, String, String) {
    let (fixture, _, _) = workspace(name);
    let older = older("2026-10-01", "accepted").replace("fd-000002", "fd-7c2e41");
    fs::create_dir_all(fixture.path("work/docs/search/decisions")).unwrap();
    fs::write(fixture.path(BESIDE_OLDER_FILE), &older).unwrap();
    let draft = DRAFT
        .replace("fd-000002", "fd-7c2e41")
        .replace("topic:", "supersedes: dc-0000001\ntopic:");
    (fixture, older, draft)
}

/// A factory run: the code repo reaches the docs through a pointer, the docs clone is a
/// read-only mount, and the draft supersedes a record in it.
#[test]
fn unattended_leaves_the_record_it_supersedes_in_a_read_only_docs_clone_as_it_is() {
    let (fixture, older, draft) = beside_the_docs("decide-sandbox-supersedes");
    set_read_only(&fixture.path("work/docs"), true);

    let args = ["decide", "--unattended", "--file", "-"];
    let run = fixture.run_with("work/web", &args, &draft, &[DATE]);

    set_read_only(&fixture.path("work/docs"), false);
    assert_eq!(
        run.stderr,
        "warning: `dc-0000001` in ../docs/search/decisions/2026-10-01-page-size.md isn't marked \
         superseded, because this run leaves the tree's records as they are: mark it when the \
         record moves into the docs\n"
    );
    let path = ".manifestdocs/decisions/2026-10-29-page-size.md";
    assert_eq!(run.stdout, format!("{path}\n"));
    assert_eq!(run.code, 0);
    assert_eq!(fixture.read(BESIDE_OLDER_FILE), older);
    let text = fixture.read(&format!("work/web/{path}"));
    let expected = superseding_record(&id_in(&text))
        .replace("fd-000002", "fd-7c2e41")
        .replace("status: accepted", "status: proposed");
    assert_eq!(text, expected);
}

#[test]
fn json_names_no_superseded_record_when_out_is_outside_the_tree() {
    let (fixture, older, draft) = beside_the_docs("decide-json-out-supersedes");

    let args = ["decide", "--out", "drafts", "--json", "--file", "-"];
    let run = fixture.run_with("work/web", &args, &draft, &[DATE]);

    let data = &parsed(&run)["data"];
    assert_eq!(data["outcome"], "written");
    assert_eq!(data["file"], "drafts/2026-10-29-page-size.md");
    assert_eq!(data["superseded"], Value::Null);
    assert_eq!(data["warnings"][0]["code"], "superseded_not_marked");
    assert_eq!(fixture.read(BESIDE_OLDER_FILE), older);
    assert_eq!(run.code, 0);
}

#[test]
fn unattended_marks_a_draft_it_supersedes_in_the_drafts_folder() {
    let fixture = cookbook(
        "decide-supersedes-draft",
        "name: demo\n",
        &[("draft.md", DRAFT)],
    );
    decide(&fixture, &["--file", "draft.md", "--unattended"]);
    let first = ".manifestdocs/decisions/2026-10-29-page-size.md";
    let first_id = id_in(&fixture.read(first));
    let second = DRAFT
        .replace("topic:", &format!("supersedes: {first_id}\ntopic:"))
        .replace("20 recipes", "25 recipes");

    let args = ["decide", "--unattended", "--file", "-"];
    let run = fixture.run_with("", &args, &second, &[DATE]);

    assert_eq!(
        run.stdout,
        format!(".manifestdocs/decisions/2026-10-29-page-size-2.md\nsuperseded: {first}\n")
    );
    assert_eq!(run.stderr, "");
    assert_eq!(run.code, 0);
    assert!(fixture.read(first).contains("\nstatus: superseded\n"));
}

#[test]
fn out_into_the_tree_marks_the_record_it_supersedes() {
    let fixture = superseding("decide-out-in-tree", OLDER_FILE, "2026-10-01");

    let run = decide(
        &fixture,
        &["--file", "draft.md", "--out", "search/decisions"],
    );

    assert_eq!(run.stdout, format!("{WRITTEN}\nsuperseded: {OLDER_FILE}\n"));
    assert_eq!(run.stderr, "");
    assert_eq!(run.code, 0);
    assert_eq!(fixture.read(OLDER_FILE), older("2026-10-01", "superseded"));
    assert_eq!(fixture.check("").stdout, "2 nodes: no problems\n");
}
