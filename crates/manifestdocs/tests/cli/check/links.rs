use crate::support::{Fixture, node};

/// A valid node whose body continues with `body`, starting on line 9.
fn node_with_body(id: &str, title: &str, body: &str) -> String {
    format!("{}{body}", node(id, title))
}

#[test]
fn reports_broken_links_in_nodes_and_supporting_docs_at_their_lines() {
    let root = node_with_body(
        "fd-000001",
        "Demo",
        "[search](search/search.md)\n[engine ADR](../adr/0001-engine.md)\n[folder](search/)\n\
         [tips](Kitchen%20Notes/tips.md)\n[readme](/README.md)\n[site](https://example.com/gone.md)\n\
         `[code](nope.md)`\n![logo](logo.png)\n[ref][r]\n![missing](missing.png)\n[ref2][r2]\n\n\
         [r]: search/search.md\n[r2]: gone.md\n",
    );
    let search = node_with_body(
        "fd-000002",
        "Search",
        "[gone](ranking.md)\n[case](../Search/search.md)\n[above](../../../x.md)\n",
    );
    let broken = "---\nid: fd-000003\ntitle: Broken\nstatus: draft\nsummary: \"Broken summary.\"\n---\n\n[x](nowhere.md)\n";
    let record = "---\nid: dc-0000001\nnode: fd-000002\ntitle: \"Engine\"\nstatus: accepted\ntopic: engine\ndate: 2026-10-28\n---\n\n[x](nope.md)\n";
    let fixture = Fixture::new(
        "links",
        &[
            ("README.md", "# Readme\n"),
            ("adr/0001-engine.md", "# Engine\n"),
            ("docs/manifestdocs.yaml", "name: demo\n"),
            ("docs/demo.md", &root),
            ("docs/logo.png", "png"),
            ("docs/Kitchen Notes/tips.md", "# Tips\n"),
            ("docs/notes.md", "See [missing](missing.md).\n"),
            ("docs/search/search.md", &search),
            ("docs/search/decisions/2026-10-28-engine.md", record),
            ("docs/broken/broken.md", broken),
        ],
    );

    let run = fixture.check("docs");

    assert_eq!(
        run.stdout,
        "broken/broken.md:3: error: `title` must be double-quoted\n\
         broken/broken.md:8: error: broken link `nowhere.md`\n\
         demo.md:18: error: broken link `missing.png`\n\
         demo.md:19: error: broken link `gone.md`\n\
         notes.md:1: error: broken link `missing.md`\n\
         search/search.md:9: error: broken link `ranking.md`\n\
         search/search.md:10: error: broken link `../Search/search.md`\n\
         search/search.md:11: error: broken link `../../../x.md`\n\
         \n3 nodes: 8 errors\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn checks_links_in_a_node_file_whose_frontmatter_never_closes() {
    let root = node("fd-000001", "Demo");
    let fixture = Fixture::new(
        "links-unclosed",
        &[
            ("manifestdocs.yaml", "name: demo\n"),
            ("demo.md", &root),
            ("feature/feature.md", "---\nid: fd-000002\n\n[x](gone.md)\n"),
        ],
    );

    let run = fixture.check("");

    assert_eq!(
        run.stdout,
        "feature/feature.md:1: error: frontmatter isn't valid YAML: no closing `---` line\n\
         feature/feature.md:4: error: broken link `gone.md`\n\
         \n2 nodes: 2 errors\n"
    );
}

#[test]
fn skips_links_it_cant_check_and_reports_bad_encoding() {
    let root = node_with_body(
        "fd-000001",
        "Demo",
        "[bad](bad%FF.md)\n<https://example.com>\n<sam@example.com>\n[mail](mailto:sam@example.com)\n\
         [cdn](//cdn.example.com/x.png)\n[raw](logo.png?raw=true)\n[top](#)\n[d][missing]\n\n\
         ```\n[fenced](nope.md)\n```\n\n    [indented](nope.md)\n\n[unused]: nope.md\n",
    );
    let fixture = Fixture::new(
        "links-skipped",
        &[
            ("manifestdocs.yaml", "name: demo\n"),
            ("demo.md", &root),
            ("logo.png", "png"),
        ],
    );

    let run = fixture.check("");

    assert_eq!(
        run.stdout,
        "demo.md:9: error: broken link `bad%FF.md`\n\n1 node: 1 error\n"
    );
}
