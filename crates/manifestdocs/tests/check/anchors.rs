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
         [nested](vendor/guide.md#setup)\n\
         [nested-missing](vendor/guide.md#nowhere)\n\
         [record-missing](search/decisions/2026-10-28-engine.md#why)\n\
         [image-heading](search/search.md#-project)\n\
         [single-quoted](search/search.md#two)\n\
         [unquoted](search/search.md#three)\n\
         [data-attribute](search/search.md#four)\n\
         [comment](search/search.md#five)\n\
         [source-line](search/search.md?plain=1#L2)\n"
    );
    let search = format!(
        "{}{}",
        node("fd-000002", "Search"),
        "\n## Write path\n\n<a id=\"step-one\"></a>\n\n## Notes\n\n## Notes\n\n\
         # ![logo](logo.png) Project\n\n\
         <span name='two'>x</span> <a ID=three>y</a>\n\n\
         <div data-id=\"four\"></div>\n\n\
         <!-- id=\"five\" -->\n"
    );
    let fixture = Fixture::new(
        "anchors",
        &[
            ("adr/0001-engine.md", "# Engine choice\n"),
            ("src/main.rs", "fn main() {}\n"),
            ("docs/manifestdocs.yaml", "name: demo\n"),
            ("docs/demo.md", &root),
            ("docs/search/search.md", &search),
            ("docs/search/logo.png", "png"),
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
         demo.md:21: error: no heading `#nowhere` in `vendor/guide.md`\n\
         demo.md:22: error: no heading `#why` in `search/decisions/2026-10-28-engine.md`\n\
         demo.md:26: error: no heading `#four` in `search/search.md`\n\
         demo.md:27: error: no heading `#five` in `search/search.md`\n\
         vendor/: warning: skipped nested tree\n\
         \n2 nodes: 8 errors, 1 warning\n"
    );
    assert_eq!(run.code, 1);
}

#[test]
fn matches_github_heading_anchors() {
    let guide = "# Write path\n\n## A & B\n\n## Why `check` exits 1\n\n## snake_case and kebab-case\n\n\
                 ## Q&A: what's next?\n\n## Café crème\n\n## 日本語の見出し\n\n## v1.0 (beta)\n\n\
                 Setup steps\n---\n\n## Notes\n\n## Notes\n\n## Notes-1\n\n## Notes\n\n\
                 ```\n# Not a heading\n```\n";
    let links = [
        "write-path",
        "a--b",
        "why-check-exits-1",
        "snake_case-and-kebab-case",
        "qa-whats-next",
        "caf%C3%A9-cr%C3%A8me",
        "日本語の見出し",
        "v10-beta",
        "setup-steps",
        "notes",
        "notes-1",
        "notes-1-1",
        "notes-2",
        "not-a-heading",
    ];
    let body: String = links
        .iter()
        .map(|anchor| format!("[x](guide.md#{anchor})\n"))
        .collect();
    let root = format!("{}{body}", node("fd-000001", "Demo"));
    let fixture = Fixture::new(
        "slugs",
        &[
            ("manifestdocs.yaml", "name: demo\n"),
            ("demo.md", &root),
            ("guide.md", guide),
        ],
    );

    let run = fixture.check("");

    assert_eq!(
        run.stdout,
        "demo.md:22: error: no heading `#not-a-heading` in `guide.md`\n\n1 node: 1 error\n"
    );
}
