use crate::support::{Fixture, node};

#[test]
fn reports_anchors_that_match_no_heading_inside_and_outside_the_tree() {
    let root = format!(
        "{}{}",
        node("fd-000001", "Demo"),
        "[ok](search/search.md#write-path)\n\
         [case](search/search.md#Write-Path)\n\
         [missing](search/search.md#read-path)\n\
         [self](#demo)\n\
         [self-missing](#nowhere)\n\
         [outside](../adr/0001-engine.md#engine-choice)\n\
         [outside-missing](../adr/0001-engine.md#cost)\n\
         [html](search/search.md#step-one)\n\
         [repeat](search/search.md#notes-1)\n\
         [source](../src/main.rs#L10)\n\
         [record](search/decisions/2026-10-28-engine.md#decision)\n\
         [nested](vendor/guide.md#setup)\n"
    );
    let search = format!(
        "{}{}",
        node("fd-000002", "Search"),
        "\n## Write path\n\n<a id=\"step-one\"></a>\n\n## Notes\n\n## Notes\n"
    );
    let fixture = Fixture::new(
        "anchors",
        &[
            ("adr/0001-engine.md", "# Engine choice\n"),
            ("src/main.rs", "fn main() {}\n"),
            ("docs/manifestdocs.yaml", "name: demo\n"),
            ("docs/demo.md", &root),
            ("docs/search/search.md", &search),
            (
                "docs/search/decisions/2026-10-28-engine.md",
                "---\nid: dc-0000001\n---\n\n## Decision\n",
            ),
            ("docs/vendor/manifestdocs.yaml", "name: vendor\n"),
            ("docs/vendor/guide.md", "# Setup\n"),
        ],
    );

    let run = fixture.check("docs");

    assert_eq!(
        run.stdout,
        "demo.md:10: error: no heading `#Write-Path` in `search/search.md`\n\
         demo.md:11: error: no heading `#read-path` in `search/search.md`\n\
         demo.md:13: error: no heading `#nowhere` in `demo.md`\n\
         demo.md:15: error: no heading `#cost` in `../adr/0001-engine.md`\n\
         vendor/: warning: skipped nested tree\n\
         \n2 nodes: 4 errors, 1 warning\n"
    );
    assert_eq!(run.code, 1);
}
