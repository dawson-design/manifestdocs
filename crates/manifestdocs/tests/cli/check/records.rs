use crate::support::{Fixture, Run, node};

/// A valid record for the Search node, with its fields on lines 2 to 7.
fn record(id: &str, topic: &str, status: &str) -> String {
    format!(
        "---\nid: {id}\nnode: fd-000002\ntitle: \"Choose {topic}\"\nstatus: {status}\ntopic: {topic}\ndate: 2026-10-28\n---\n\n**Decision.** Use {topic}.\n"
    )
}

/// Checks a tree with a root node, a Search node and `records` at their paths.
fn check_records(name: &str, records: &[(&str, &str)]) -> Run {
    let root = node("fd-000001", "Demo");
    let search = node("fd-000002", "Search");
    let mut files = vec![
        ("manifestdocs.yaml", "name: demo\n"),
        ("demo.md", root.as_str()),
        ("search/search.md", search.as_str()),
    ];
    files.extend_from_slice(records);
    Fixture::new(name, &files).check("")
}

#[test]
fn passes_valid_records() {
    let full = "---\nid: dc-3f1a8c2\nnode: fd-000002\ntitle: \"Search results show 20 recipes per page\"\nstatus: accepted\ntopic: search-page-size\ndecided_by: octocat\ndate: 2026-10-28\nsource: [example-org/cookbook-web#212]\napplies_to: [example-org/cookbook-web:src/search/page.ts]\nsupersedes: dc-1a7f3b4\n---\n\n**Decision.** Search shows 20 recipes per page.\n";
    let older = record("dc-1a7f3b4", "search-page-size", "superseded");
    let at_root = "---\nid: dc-0000001\nnode: fd-000001\ntitle: \"No GitHub writes\"\nstatus: proposed\ntopic: github-writes\ndate: 2026-10-28\nsupersedes: null\n---\n";
    let rejected = record("dc-0000002", "engine", "rejected");

    let run = check_records(
        "records-valid",
        &[
            ("search/decisions/2026-10-28-page-size.md", full),
            ("search/decisions/2026-10-01-page-size.md", &older),
            ("search/decisions/2026-10-02-engine.md", &rejected),
            ("decisions/2026-10-28-github.md", at_root),
        ],
    );

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(run.code, 0);
}

#[test]
fn reports_each_record_frontmatter_problem_at_its_line() {
    let file = "search/decisions/2026-10-28-engine.md";
    let cases = [
        (
            "records-unquoted-title",
            "---\nid: dc-0000001\nnode: fd-000002\ntitle: Engine\nstatus: accepted\ntopic: engine\ndate: 2026-10-28\n---\n",
            "4: error: `title` must be double-quoted",
        ),
        (
            "records-missing-topic",
            "---\nid: dc-0000001\nnode: fd-000002\ntitle: \"Engine\"\nstatus: accepted\ndate: 2026-10-28\n---\n",
            "1: error: missing `topic`",
        ),
        (
            "records-empty-node",
            "---\nid: dc-0000001\nnode:\ntitle: \"Engine\"\nstatus: accepted\ntopic: engine\ndate: 2026-10-28\n---\n",
            "3: error: `node` is empty",
        ),
        (
            "records-malformed-id",
            "---\nid: dc-12\nnode: fd-000002\ntitle: \"Engine\"\nstatus: accepted\ntopic: engine\ndate: 2026-10-28\n---\n",
            "2: error: malformed decision ID `dc-12`: expected `dc-` and 7 lowercase hex characters",
        ),
        (
            "records-node-id",
            "---\nid: fd-000009\nnode: fd-000002\ntitle: \"Engine\"\nstatus: accepted\ntopic: engine\ndate: 2026-10-28\n---\n",
            "2: error: malformed decision ID `fd-000009`: expected `dc-` and 7 lowercase hex characters",
        ),
        (
            "records-malformed-node",
            "---\nid: dc-0000001\nnode: fd-12\ntitle: \"Engine\"\nstatus: accepted\ntopic: engine\ndate: 2026-10-28\n---\n",
            "3: error: malformed node ID `fd-12`: expected `fd-` and 6 lowercase hex characters",
        ),
        (
            "records-unknown-status",
            "---\nid: dc-0000001\nnode: fd-000002\ntitle: \"Engine\"\nstatus: final\ntopic: engine\ndate: 2026-10-28\n---\n",
            "5: error: status `final` isn't a record status: expected `proposed`, `accepted`, `superseded` or `rejected`",
        ),
        (
            "records-topic-case",
            "---\nid: dc-0000001\nnode: fd-000002\ntitle: \"Engine\"\nstatus: accepted\ntopic: Search Engine\ndate: 2026-10-28\n---\n",
            "6: error: `topic` must be kebab-case text",
        ),
        (
            "records-date-format",
            "---\nid: dc-0000001\nnode: fd-000002\ntitle: \"Engine\"\nstatus: accepted\ntopic: engine\ndate: 28/10/2026\n---\n",
            "7: error: `date` must be a date, YYYY-MM-DD",
        ),
        (
            "records-decided-by-list",
            "---\nid: dc-0000001\nnode: fd-000002\ntitle: \"Engine\"\nstatus: accepted\ntopic: engine\ndate: 2026-10-28\ndecided_by: [sam, kim]\n---\n",
            "8: error: `decided_by` must be text",
        ),
        (
            "records-source-text",
            "---\nid: dc-0000001\nnode: fd-000002\ntitle: \"Engine\"\nstatus: accepted\ntopic: engine\ndate: 2026-10-28\nsource: acme/web#1\n---\n",
            "8: error: `source` must be a list",
        ),
        (
            "records-malformed-source",
            "---\nid: dc-0000001\nnode: fd-000002\ntitle: \"Engine\"\nstatus: accepted\ntopic: engine\ndate: 2026-10-28\nsource: [acme/web]\n---\n",
            "8: error: malformed work item `acme/web`: expected `owner/name#number`",
        ),
        (
            "records-applies-to-text",
            "---\nid: dc-0000001\nnode: fd-000002\ntitle: \"Engine\"\nstatus: accepted\ntopic: engine\ndate: 2026-10-28\napplies_to: acme/web:src/main.rs\n---\n",
            "8: error: `applies_to` must be a list",
        ),
        (
            "records-malformed-supersedes",
            "---\nid: dc-0000001\nnode: fd-000002\ntitle: \"Engine\"\nstatus: accepted\ntopic: engine\ndate: 2026-10-28\nsupersedes: 1a7f3b4\n---\n",
            "8: error: malformed decision ID `1a7f3b4`: expected `dc-` and 7 lowercase hex characters",
        ),
        (
            "records-unclosed",
            "---\nid: dc-0000001\nnode: fd-000002\n\n**Decision.**\n",
            "1: error: frontmatter isn't valid YAML: no closing `---` line",
        ),
    ];
    for (name, text, problem) in cases {
        let run = check_records(name, &[(file, text)]);

        assert_eq!(
            run.stdout,
            format!("{file}:{problem}\n\n2 nodes: 1 error\n"),
            "{name}"
        );
        assert_eq!(run.code, 1, "{name}");
    }
}

#[test]
fn ignores_a_file_without_frontmatter_in_a_decisions_folder() {
    let run = check_records(
        "records-readme",
        &[(
            "search/decisions/README.md",
            "# Decisions\n\nOne record per file.\n",
        )],
    );

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(run.code, 0);
}

#[test]
fn warns_about_unknown_fields_in_a_record() {
    let text = record("dc-0000001", "engine", "accepted")
        .replace("date: 2026-10-28\n", "date: 2026-10-28\nowner: sam\n");

    let run = check_records("records-unknown-field", &[("search/decisions/a.md", &text)]);

    assert_eq!(
        run.stdout,
        "search/decisions/a.md:8: warning: unknown field `owner`\n\n2 nodes: 1 warning\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn reports_decision_ids_used_by_two_records() {
    let first = record("dc-0000001", "engine", "accepted");
    let second = record("dc-0000001", "ranking", "accepted");

    let run = check_records(
        "records-duplicate",
        &[
            ("search/decisions/a.md", &first),
            ("search/decisions/b.md", &second),
        ],
    );

    assert_eq!(
        run.stdout,
        "search/decisions/a.md:2: error: decision ID is also used by `search/decisions/b.md`\n\
         search/decisions/b.md:2: error: decision ID is also used by `search/decisions/a.md`\n\
         \n2 nodes: 2 errors\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn reports_a_record_whose_node_doesnt_exist() {
    let text = record("dc-0000001", "engine", "accepted").replace("fd-000002", "fd-00000f");

    let run = check_records("records-no-node", &[("search/decisions/a.md", &text)]);

    assert_eq!(
        run.stdout,
        "search/decisions/a.md:3: error: `node` names `fd-00000f`, which isn't a node in this tree\n\
         \n2 nodes: 1 error\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn reports_a_record_in_another_nodes_decisions_folder() {
    let text = record("dc-0000001", "engine", "accepted");

    let run = check_records("records-wrong-folder", &[("decisions/a.md", &text)]);

    assert_eq!(
        run.stdout,
        "decisions/a.md:3: error: `node` is `fd-000002`, but the record's `decisions/` folder isn't that node's\n\
         \n2 nodes: 1 error\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn reports_a_record_in_a_decisions_folder_no_node_owns() {
    let search = node("fd-000002", "Search");
    let text = record("dc-0000001", "engine", "accepted");
    let fixture = Fixture::new(
        "records-no-owner",
        &[
            ("manifestdocs.yaml", "name: demo\n"),
            ("search/search.md", &search),
            ("decisions/a.md", &text),
        ],
    );

    let run = fixture.check("");

    assert_eq!(
        run.stdout,
        "decisions/a.md:3: error: `node` is `fd-000002`, but the record's `decisions/` folder isn't that node's\n\
         manifestdocs.yaml: warning: no root node `demo.md`\n\
         \n1 node: 1 error, 1 warning\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn gives_a_subfolder_of_decisions_to_the_node_owning_decisions() {
    let text = record("dc-0000001", "engine", "accepted");

    let run = check_records(
        "records-subfolder",
        &[("search/decisions/2026/a.md", &text)],
    );

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(run.code, 0);
}

#[test]
fn gives_a_decisions_folder_outside_a_node_folder_to_the_nearest_node_above() {
    let text = record("dc-0000001", "engine", "accepted");

    let run = check_records(
        "records-non-node-folder",
        &[("search/notes/decisions/a.md", &text)],
    );

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(run.code, 0);
}

#[test]
fn still_resolves_supersedes_to_a_record_whose_yaml_doesnt_parse() {
    let broken = "---\nid: dc-0000001\nnode: fd-000002\ntitle: \"Engine\"\ntopic: [engine\n---\n";
    let newer = record("dc-0000002", "engine", "accepted").replace(
        "date: 2026-10-28\n",
        "date: 2026-10-28\nsupersedes: dc-0000001\n",
    );

    let run = check_records(
        "records-unparsable",
        &[
            ("search/decisions/a.md", broken),
            ("search/decisions/b.md", &newer),
        ],
    );

    assert!(
        run.stdout
            .starts_with("search/decisions/a.md:5: error: frontmatter isn't valid YAML: "),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout.ends_with("\n\n2 nodes: 1 error\n"),
        "{}",
        run.stdout
    );
    assert_eq!(run.code, 1);
}

#[test]
fn reports_a_supersedes_that_names_no_record() {
    let text = format!(
        "{}supersedes: dc-00000ff\n---\n",
        record("dc-0000001", "engine", "accepted")
            .split("---\n\n")
            .next()
            .unwrap()
    );

    let run = check_records(
        "records-no-predecessor",
        &[("search/decisions/a.md", &text)],
    );

    assert_eq!(
        run.stdout,
        "search/decisions/a.md:8: error: `supersedes` names `dc-00000ff`, which isn't a decision record in this tree\n\
         \n2 nodes: 1 error\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn reports_two_accepted_records_with_one_topic_in_a_node() {
    let first = record("dc-0000001", "engine", "accepted");
    let second = record("dc-0000002", "engine", "accepted");

    let run = check_records(
        "records-topic-twice",
        &[
            ("search/decisions/a.md", &first),
            ("search/decisions/b.md", &second),
        ],
    );

    assert_eq!(
        run.stdout,
        "search/decisions/a.md:6: error: another accepted record, `search/decisions/b.md`, has topic `engine`\n\
         search/decisions/b.md:6: error: another accepted record, `search/decisions/a.md`, has topic `engine`\n\
         \n2 nodes: 2 errors\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn allows_one_topic_in_two_nodes_or_once_accepted_in_a_node() {
    let accepted = record("dc-0000001", "engine", "accepted");
    let superseded = record("dc-0000002", "engine", "superseded");
    let proposed = record("dc-0000003", "engine", "proposed");
    let at_root = record("dc-0000004", "engine", "accepted").replace("fd-000002", "fd-000001");

    let run = check_records(
        "records-topic-once",
        &[
            ("search/decisions/a.md", &accepted),
            ("search/decisions/b.md", &superseded),
            ("search/decisions/c.md", &proposed),
            ("decisions/d.md", &at_root),
        ],
    );

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(run.code, 0);
}

#[test]
fn warns_about_a_record_body_over_300_tokens() {
    let frontmatter = record("dc-0000001", "engine", "accepted");
    let frontmatter = frontmatter.split("\n\n").next().unwrap();
    // 900 characters estimate at 300 tokens, the limit; one more goes over.
    let at_limit = format!("{frontmatter}\n\n{}\n", "x".repeat(900));
    let over = format!("{frontmatter}\n\n{}\n", "x".repeat(901));

    let fine = check_records(
        "records-size-at-limit",
        &[("search/decisions/a.md", &at_limit)],
    );
    let large = check_records("records-size-over", &[("search/decisions/a.md", &over)]);

    assert_eq!(fine.stdout, "2 nodes: no problems\n");
    assert_eq!(
        large.stdout,
        "search/decisions/a.md: warning: record body is about 301 tokens, over 300; a choice that needs more is a spec change or an ADR\n\
         \n2 nodes: 1 warning\n"
    );
    assert_eq!(large.code, 0);
}
