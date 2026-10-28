use crate::support::{Fixture, node};

const CONFIG: (&str, &str) = ("manifestdocs.yaml", "name: demo\n");

fn superseded(id: &str, by: &str) -> String {
    format!(
        "---\nid: {id}\ntitle: \"Old\"\nstatus: superseded\nsummary: \"Old summary.\"\nsuperseded_by: {by}\n---\n"
    )
}

#[test]
fn reports_every_file_that_shares_an_id() {
    let root = node("fd-000001", "Demo");
    let shared = node("fd-000002", "Shared");
    let fixture = Fixture::new(
        "duplicate-ids",
        &[
            CONFIG,
            ("demo.md", &root),
            ("a/a.md", &shared),
            ("b/b.md", &shared),
            ("c/c.md", &shared),
        ],
    );

    let run = fixture.check("");

    assert_eq!(
        run.stdout,
        "a/a.md:2: error: ID is also used by `b/b.md`\n\
         b/b.md:2: error: ID is also used by `a/a.md`\n\
         c/c.md:2: error: ID is also used by `a/a.md`\n\
         \n4 nodes: 3 errors\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn reports_a_replacement_that_names_no_node() {
    let root = node("fd-000001", "Demo");
    let old = superseded("fd-000002", "fd-0000ff");
    let fixture = Fixture::new(
        "missing-replacement",
        &[CONFIG, ("demo.md", &root), ("old/old.md", &old)],
    );

    let run = fixture.check("");

    assert_eq!(
        run.stdout,
        "old/old.md:6: error: `superseded_by` names `fd-0000ff`, which isn't a node in this tree\n\n2 nodes: 1 error\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn resolves_a_replacement_whose_own_frontmatter_has_problems() {
    let root = node("fd-000001", "Demo");
    let old = superseded("fd-000002", "fd-000003");
    let new = "---\nid: fd-000003\ntitle: \"New\"\nstatus: live\nsummary: \"New summary.\"\n---\n";
    let fixture = Fixture::new(
        "replacement-with-problems",
        &[
            CONFIG,
            ("demo.md", &root),
            ("old/old.md", &old),
            ("new/new.md", new),
        ],
    );

    let run = fixture.check("");

    assert_eq!(
        run.stdout,
        "new/new.md:4: error: status `live` isn't in `statuses` in manifestdocs.yaml\n\n3 nodes: 1 error\n"
    );
}

#[test]
fn reports_a_node_id_in_a_file_that_isnt_a_node_file() {
    let root = node("fd-000001", "Demo");
    let renamed = node("fd-000002", "Search");
    let fixture = Fixture::new(
        "stray-ids",
        &[
            CONFIG,
            ("demo.md", &root),
            ("search-v2/search.md", &renamed),
            ("notes.md", "---\nid: \"fd-000009\"\n---\n# Notes\n"),
            ("old-name/ranking.md", "---\nid: fd-00000a # renamed\n---\n"),
        ],
    );

    let run = fixture.check("");

    assert_eq!(
        run.stdout,
        "notes.md:2: error: node ID `fd-000009` in a file that isn't named after its folder\n\
         old-name/ranking.md:2: error: node ID `fd-00000a` in a file that isn't named after its folder\n\
         search-v2/search.md:2: error: node ID `fd-000002` in a file that isn't named after its folder\n\
         \n1 node: 3 errors\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn skips_decision_records() {
    let root = node("fd-000001", "Demo");
    let record = "---\nid: dc-0000001\nnode: fd-000001\ntitle: Unquoted\nowner: sam\n---\n";
    let fixture = Fixture::new(
        "records",
        &[
            CONFIG,
            ("demo.md", &root),
            ("decisions/2026-10-28-search.md", record),
            ("decisions/notes.md", "---\nid: fd-000009\n---\n"),
        ],
    );

    let run = fixture.check("");

    assert_eq!(run.stdout, "1 node: no problems\n");
    assert_eq!(run.code, 0);
}
