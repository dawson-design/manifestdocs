use crate::support::{Fixture, Run, node};

/// Checks a tree with a valid root node and one feature node, `feature/feature.md`.
fn check_feature(name: &str, feature: &str) -> Run {
    let root = node("fd-000001", "Demo");
    let fixture = Fixture::new(
        name,
        &[
            ("manifestdocs.yaml", "name: demo\n"),
            ("demo.md", &root),
            ("feature/feature.md", feature),
        ],
    );
    fixture.check("")
}

#[test]
fn reports_each_frontmatter_problem_at_its_line() {
    let cases = [
        (
            "unquoted-title",
            "---\nid: fd-000002\ntitle: Feature\nstatus: draft\nsummary: \"Feature summary.\"\n---\n",
            "feature/feature.md:3: error: `title` must be double-quoted\n\n2 nodes: 1 error\n",
        ),
        (
            "title-with-hash",
            "---\nid: fd-000002\ntitle: Feature #2\nstatus: draft\nsummary: \"Feature summary.\"\n---\n",
            "feature/feature.md:3: error: `title` must be double-quoted\n\n2 nodes: 1 error\n",
        ),
        (
            "single-quoted-summary",
            "---\nid: fd-000002\ntitle: \"Feature\"\nstatus: draft\nsummary: 'Feature summary.'\n---\n",
            "feature/feature.md:5: error: `summary` must be double-quoted\n\n2 nodes: 1 error\n",
        ),
        (
            "missing-summary",
            "---\nid: fd-000002\ntitle: \"Feature\"\nstatus: draft\n---\n",
            "feature/feature.md:1: error: missing `summary`\n\n2 nodes: 1 error\n",
        ),
        (
            "empty-title",
            "---\nid: fd-000002\ntitle: \"\"\nstatus: draft\nsummary: \"Feature summary.\"\n---\n",
            "feature/feature.md:3: error: `title` is empty\n\n2 nodes: 1 error\n",
        ),
        (
            "malformed-id",
            "---\nid: fd-12\ntitle: \"Feature\"\nstatus: draft\nsummary: \"Feature summary.\"\n---\n",
            "feature/feature.md:2: error: malformed node ID `fd-12`: expected `fd-` and 6 lowercase hex characters\n\n1 node: 1 error\n",
        ),
        (
            "repos-not-a-list",
            "---\nid: fd-000002\ntitle: \"Feature\"\nstatus: draft\nsummary: \"Feature summary.\"\nrepos: acme/cookbook-web\n---\n",
            "feature/feature.md:6: error: `repos` must be a list\n\n2 nodes: 1 error\n",
        ),
        (
            "malformed-repo",
            "---\nid: fd-000002\ntitle: \"Feature\"\nstatus: draft\nsummary: \"Feature summary.\"\nrepos: [cookbook-web]\n---\n",
            "feature/feature.md:6: error: malformed repo `cookbook-web`: expected `owner/name`\n\n2 nodes: 1 error\n",
        ),
        (
            "malformed-work-item",
            "---\nid: fd-000002\ntitle: \"Feature\"\nstatus: draft\nsummary: \"Feature summary.\"\nwork:\n  - acme/cookbook-docs#x\n---\n",
            "feature/feature.md:6: error: malformed work item `acme/cookbook-docs#x`: expected `owner/name#number`\n\n2 nodes: 1 error\n",
        ),
        (
            "unknown-status",
            "---\nid: fd-000002\ntitle: \"Feature\"\nstatus: live\nsummary: \"Feature summary.\"\n---\n",
            "feature/feature.md:4: error: status `live` isn't in `statuses` in manifestdocs.yaml\n\n2 nodes: 1 error\n",
        ),
        (
            "superseded-without-replacement",
            "---\nid: fd-000002\ntitle: \"Feature\"\nstatus: superseded\nsummary: \"Feature summary.\"\n---\n",
            "feature/feature.md:4: error: status is `superseded` but `superseded_by` is missing\n\n2 nodes: 1 error\n",
        ),
        (
            "replacement-without-superseded",
            "---\nid: fd-000002\ntitle: \"Feature\"\nstatus: draft\nsummary: \"Feature summary.\"\nsuperseded_by: fd-000001\n---\n",
            "feature/feature.md:6: error: `superseded_by` is set but status isn't `superseded`\n\n2 nodes: 1 error\n",
        ),
        (
            "superseded-by-self",
            "---\nid: fd-000002\ntitle: \"Feature\"\nstatus: superseded\nsummary: \"Feature summary.\"\nsuperseded_by: fd-000002\n---\n",
            "feature/feature.md:6: error: `superseded_by` names this node\n\n2 nodes: 1 error\n",
        ),
        (
            "unclosed",
            "---\nid: fd-000002\ntitle: \"Feature\"\n\n# Feature\n",
            "feature/feature.md:1: error: frontmatter isn't valid YAML: no closing `---` line\n\n2 nodes: 1 error\n",
        ),
    ];
    for (name, feature, expected) in cases {
        let run = check_feature(name, feature);
        assert_eq!(run.stdout, expected, "{name}");
        assert_eq!(run.code, 1, "{name}");
    }
}

#[test]
fn reports_yaml_that_doesnt_parse_and_still_counts_the_node() {
    let feature = "---\nid: fd-000002\nid: fd-000003\ntitle: \"Feature\"\n---\n";

    let run = check_feature("duplicate-key", feature);

    assert!(
        run.stdout
            .starts_with("feature/feature.md:3: error: frontmatter isn't valid YAML: "),
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
fn warns_about_unknown_fields_and_node_files_without_an_id() {
    let cases = [
        (
            "unknown-field",
            "---\nid: fd-000002\ntitle: \"Feature\"\nstatus: draft\nsummary: \"Feature summary.\"\nowner: sam\n---\n",
            "feature/feature.md:6: warning: unknown field `owner`\n\n2 nodes: 1 warning\n",
        ),
        (
            "no-id",
            "---\ntitle: \"Feature\"\n---\n",
            "feature/feature.md:1: warning: node file has frontmatter but no `id`\n\n1 node: 1 warning\n",
        ),
        (
            "empty-block",
            "---\n---\n# Feature\n",
            "feature/feature.md:1: warning: node file has frontmatter but no `id`\n\n1 node: 1 warning\n",
        ),
        (
            "comment-only-block",
            "---\n# id: fd-000002\n---\n",
            "feature/feature.md:1: warning: node file has frontmatter but no `id`\n\n1 node: 1 warning\n",
        ),
    ];
    for (name, feature, expected) in cases {
        let run = check_feature(name, feature);
        assert_eq!(run.stdout, expected, "{name}");
        assert_eq!(run.code, 0, "{name}");
    }
}

#[test]
fn treats_a_node_file_without_frontmatter_as_a_supporting_doc() {
    // Frontmatter only counts when `---` is the file's first line.
    for (name, feature) in [
        ("no-frontmatter", "# Feature\n"),
        ("late-fence", "\n---\nid: fd-000002\n---\n"),
    ] {
        let run = check_feature(name, feature);
        assert_eq!(run.stdout, "1 node: no problems\n", "{name}");
        assert_eq!(run.code, 0, "{name}");
    }
}

#[test]
fn reads_valid_nodes_with_crlf_endings_a_byte_order_mark_or_a_replacement() {
    let cases = [
        (
            "crlf",
            "---\r\nid: fd-000002\r\ntitle: \"Feature\"\r\nstatus: draft\r\nsummary: \"Feature summary.\"\r\n---\r\n",
        ),
        (
            "bom",
            "\u{feff}---\nid: fd-000002\ntitle: \"Feature\"\nstatus: approved\nsummary: \"Feature summary.\"\n---\n",
        ),
        (
            "superseded",
            "---\nid: fd-000002\ntitle: \"Feature\"\nstatus: superseded\nsummary: \"Feature summary.\"\nsuperseded_by: fd-000001\nrepos: [acme/cookbook-web]\nwork: [acme/cookbook-docs#41]\n---\n",
        ),
    ];
    for (name, feature) in cases {
        let run = check_feature(name, feature);
        assert_eq!(run.stdout, "2 nodes: no problems\n", "{name}");
        assert_eq!(run.code, 0, "{name}");
    }
}

#[test]
fn reads_an_id_however_the_yaml_writes_it() {
    let cases = [
        (
            "space-before-colon",
            "---\nid : fd-000002\ntitle: \"Feature\"\nstatus: draft\nsummary: \"Feature summary.\"\n---\n",
        ),
        (
            "quoted-key",
            "---\n\"id\": fd-000002\ntitle: \"Feature\"\nstatus: draft\nsummary: \"Feature summary.\"\n---\n",
        ),
        (
            "flow-mapping",
            "---\n{id: fd-000002, title: \"Feature\", status: draft, summary: \"Feature summary.\"}\n---\n",
        ),
        (
            "comment-after-id",
            "---\nid: fd-000002 # renamed from search\ntitle: \"Feature\"\nstatus: draft\nsummary: \"Feature summary.\"\n---\n",
        ),
    ];
    for (name, feature) in cases {
        let run = check_feature(name, feature);
        assert_eq!(run.stdout, "2 nodes: no problems\n", "{name}");
        assert_eq!(run.code, 0, "{name}");
    }
}

#[test]
fn reports_every_field_problem_when_one_field_has_the_wrong_type() {
    let feature = "---\nid: fd-000002\ntitle: \"Feature\"\nstatus: live\nsummary: \"Feature summary.\"\nrepos: acme/cookbook-web\nwork: acme/cookbook-docs#41\n---\n";

    let run = check_feature("several-problems", feature);

    assert_eq!(
        run.stdout,
        "feature/feature.md:4: error: status `live` isn't in `statuses` in manifestdocs.yaml\n\
         feature/feature.md:6: error: `repos` must be a list\n\
         feature/feature.md:7: error: `work` must be a list\n\
         \n2 nodes: 3 errors\n"
    );
}

#[test]
fn reports_a_blank_value_once_and_accepts_a_value_on_the_next_line() {
    let feature =
        "---\nid: fd-000002\ntitle:\nstatus: draft\nsummary:\n  \"Feature summary.\"\n---\n";

    let run = check_feature("blank-and-next-line", feature);

    assert_eq!(
        run.stdout,
        "feature/feature.md:3: error: `title` is empty\n\n2 nodes: 1 error\n"
    );
}

#[test]
fn warns_about_a_node_body_over_the_size_limit() {
    let frontmatter = "---\nid: fd-000002\ntitle: \"Feature\"\nstatus: draft\nsummary: \"Feature summary.\"\n---\n\n";
    // 7,500 characters estimate at 2,500 tokens, the default limit; one more goes over.
    let at_limit = format!("{frontmatter}{}\n", "x".repeat(7_500));
    let over = format!("{frontmatter}{}\n", "x".repeat(7_501));

    let fine = check_feature("size-at-limit", &at_limit);
    let large = check_feature("size-over", &over);

    assert_eq!(fine.stdout, "2 nodes: no problems\n");
    assert_eq!(
        large.stdout,
        "feature/feature.md: warning: node body is about 2,501 tokens, over `node_tokens` (2,500); split it into child nodes\n\n2 nodes: 1 warning\n"
    );
    assert_eq!(large.code, 0);
}

#[test]
fn takes_the_size_limit_from_the_config() {
    let root = node("fd-000001", "Demo");
    let fixture = Fixture::new(
        "size-configured",
        &[
            ("manifestdocs.yaml", "name: demo\nnode_tokens: 1\n"),
            ("demo.md", &root),
        ],
    );

    let run = fixture.check("");

    assert_eq!(
        run.stdout,
        "demo.md: warning: node body is about 2 tokens, over `node_tokens` (1); split it into child nodes\n\n1 node: 1 warning\n"
    );
}
