//! `manifestdocs context`.

use std::fs;
use std::process::Command;

use crate::docs_source::{git as docs_git, workspace};
use crate::support::{Fixture, Run, node};

const SEARCH: &str = "---\nid: fd-7c2e41\ntitle: \"Search\"\nstatus: draft\nsummary: \"Finds recipes.\"\n---\n\n\
# Search\n\n\
Ranks by the [ranking rules](ranking-rules.md) and the [engine ADR](../../../adr/engine.md).\n\
Again: [rules](ranking-rules.md#ties), the [project](../../cookbook.md) and [this page](#search).\n\
See [filters](filters/), [import](../import/import.md), ![flow](flow.png), [guidelines](https://example.com/g) and [gone](gone.md).\n\
`[code](code.md)`\n";

/// Links to a doc outside the tree, which loads, and to nodes already in the context, which
/// aren't listed again.
const RANKING: &str = "# Ranking rules\n\n\
Ties go to [newer](../../glossary.md), and see the [engine](../../../adr/engine.md), \
[recipes](../recipes.md), [import](../import/import.md) and [filters](filters/filters.md).\n";

const ENGINE: &str = "# Engine\n\nUses [the index](https://example.com/index).\n";

/// A tree in `docs/` whose Search node links to supporting docs, an ADR outside the tree,
/// other nodes, an image, a URL and a missing file.
fn cookbook(name: &str, engine: &str) -> Fixture {
    let root = node("fd-0a0001", "Cookbook");
    let recipes = node("fd-0a0002", "Recipes");
    let import = node("fd-0a0003", "Import");
    let filters = node("fd-7c2e99", "Filters");
    Fixture::new(
        name,
        &[
            ("README.md", "# Cookbook repo\n"),
            ("adr/engine.md", engine),
            ("docs/manifestdocs.yaml", "name: cookbook\n"),
            ("docs/cookbook.md", &root),
            ("docs/glossary.md", "# Glossary\n"),
            ("docs/recipes/recipes.md", &recipes),
            ("docs/recipes/import/import.md", &import),
            ("docs/recipes/search/search.md", SEARCH),
            ("docs/recipes/search/ranking-rules.md", RANKING),
            ("docs/recipes/search/flow.png", "png"),
            ("docs/recipes/search/code.md", "# Code\n"),
            ("docs/recipes/search/filters/filters.md", &filters),
        ],
    )
}

/// The text before the `Estimated size` line, and the size it states.
fn estimate(run: &Run) -> (&str, &str) {
    let (before, line) = run
        .stdout
        .rsplit_once("Estimated size: ")
        .unwrap_or_else(|| panic!("no estimate: {}", run.stdout));
    (before, line.trim_end())
}

/// The names in the `Omitted for budget` line, without their sizes.
fn omitted(run: &Run) -> Vec<&str> {
    let (before, _) = estimate(run);
    let Some((_, list)) = before.rsplit_once("Omitted for budget: ") else {
        return Vec::new();
    };
    let items = list.trim_end().split(", ");
    items.map(|item| item.split(" (").next().unwrap()).collect()
}

#[test]
fn assembles_a_node_with_its_ancestors_linked_docs_and_children() {
    let fixture = cookbook("context-full", ENGINE);

    let run = fixture.run("docs", &["context", "fd-7c2e41"]);

    let body = SEARCH.split_once("---\n\n").unwrap().1.trim_end();
    let ranking = RANKING.trim_end();
    let expected = format!(
        "# Context: fd-7c2e41 (docs not in git)\n\n\
         ## Feature: Search (fd-7c2e41, draft)\nPath: recipes/search\n\n{body}\n\n\
         ## Parent: Recipes (fd-0a0002, draft)\nPath: recipes\n\n# Recipes\n\n\
         ## Project: Cookbook (fd-0a0001, draft)\n\n# Cookbook\n\n\
         ## Linked docs\n\n\
         ### recipes/search/ranking-rules.md\n\n{ranking}\n\n\
         ### ../adr/engine.md\n\n# Engine\n\nUses [the index](https://example.com/index).\n\n\
         ## Linked nodes\n\n- Import (fd-0a0003, draft): Import summary.\n\n\
         ## Other links\n\n- recipes/search/flow.png\n- https://example.com/g\n- glossary.md\n\
         - https://example.com/index\n\n\
         ## Around this feature\n\n\
         - Child: Filters (fd-7c2e99, draft): Filters summary.\n\n"
    );
    let (before, size) = estimate(&run);
    assert_eq!(before, expected);
    // The estimate is characters divided by 3, rounded up.
    let tokens = before.chars().count().div_ceil(3);
    assert_eq!(size, format!("{tokens} tokens"));
    assert_eq!(run.code, 0);
}

/// The plugin's skills run `context` from the repo root, through the pointer, with
/// `--format md` and a budget; paths then read from the repo root.
#[test]
fn runs_through_a_pointer_as_the_skills_call_it() {
    let fixture = cookbook("context-pointer", ENGINE);
    fs::create_dir_all(fixture.path(".manifestdocs")).unwrap();
    fs::write(fixture.path(".manifestdocs/config.yaml"), "tree: docs\n").unwrap();

    let in_tree = fixture.run("docs", &["context", "fd-7c2e41"]);
    let skills = fixture.run(
        "",
        &["context", "fd-7c2e41", "--format", "md", "--budget", "8000"],
    );

    let expected = estimate(&in_tree)
        .0
        .replace("Path: recipes", "Path: docs/recipes")
        .replace("### recipes/", "### docs/recipes/")
        .replace("### ../adr/", "### adr/")
        .replace(
            "- recipes/search/flow.png",
            "- docs/recipes/search/flow.png",
        )
        .replace("- glossary.md", "- docs/glossary.md");
    let (before, size) = estimate(&skills);
    assert_eq!(before, expected);
    assert!(size.ends_with(" of 8,000 tokens"), "{size}");
    assert_eq!(skills.code, 0);
}

#[test]
fn includes_every_ancestor_in_full_even_one_with_errors() {
    let fixture = cookbook("context-ancestors", ENGINE);
    let broken = "---\nid: fd-0a0002\ntitle: Recipes\nstatus: draft\nsummary: \"Recipes summary.\"\n---\n\n\
                  # Recipes\n\nRULE: every recipe lists allergens.\n";
    fs::write(fixture.path("docs/recipes/recipes.md"), broken).unwrap();

    let run = fixture.run("docs", &["context", "fd-7c2e99"]);

    let headings: Vec<&str> = run
        .stdout
        .lines()
        .filter(|line| line.starts_with("## "))
        .collect();
    assert_eq!(
        headings,
        [
            "## Feature: Filters (fd-7c2e99, draft)",
            "## Parent: Search (fd-7c2e41, draft)",
            "## Ancestor: recipes (fd-0a0002, has problems)",
            "## Project: Cookbook (fd-0a0001, draft)",
        ]
    );
    assert!(
        run.stdout.contains(
            "## Ancestor: recipes (fd-0a0002, has problems)\nPath: recipes\n\n# Recipes\n\nRULE: every recipe lists allergens.\n"
        ),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout.contains("Ranks by the [ranking rules]"),
        "{}",
        run.stdout
    );
}

#[test]
fn leaves_out_what_doesnt_fit_the_budget_and_names_it() {
    let engine = format!("# Engine\n\n{}\n", "The engine ranks recipes. ".repeat(200));
    let fixture = cookbook("context-budget", &engine);

    let run = fixture.run("docs", &["context", "recipes/search", "--budget", "1000"]);

    let (before, size) = estimate(&run);
    assert!(!before.contains("### ../adr/engine.md"), "{before}");
    assert_eq!(omitted(&run), ["../adr/engine.md"]);
    // Pieces after the one that didn't fit still go in.
    assert!(
        before.contains("- Child: Filters (fd-7c2e99, draft)"),
        "{before}"
    );
    let (used, budget) = size.split_once(" of ").unwrap();
    assert_eq!(budget, "1,000 tokens");
    assert!(used.parse::<u32>().unwrap() <= 900, "{size}");
}

#[test]
fn always_includes_the_node_itself_and_lists_the_rest_in_fill_order() {
    let fixture = cookbook("context-tiny-budget", ENGINE);

    let run = fixture.run("docs", &["context", "fd-7c2e4", "--budget", "10"]);

    assert!(
        run.stdout.contains("## Feature: Search (fd-7c2e41, draft)"),
        "{}",
        run.stdout
    );
    assert_eq!(
        omitted(&run),
        [
            "recipes/recipes.md",
            "cookbook.md",
            "recipes/search/ranking-rules.md",
            "../adr/engine.md",
            "recipes/import/import.md",
            "recipes/search/flow.png",
            "https://example.com/g",
            "glossary.md",
            "https://example.com/index",
            "recipes/search/filters/filters.md",
        ]
    );
}

#[test]
fn counts_the_list_of_what_it_left_out_against_the_budget() {
    let summary =
        "Covers one part of the topic in enough words that its summary line is long. ".repeat(2);
    let children: Vec<(String, String)> = (1..=30)
        .map(|i| {
            let path = format!("docs/topic/c{i:02}/c{i:02}.md");
            let text = format!(
                "---\nid: fd-0c00{i:02}\ntitle: \"C{i:02}\"\nstatus: draft\nsummary: \"{}\"\n---\n",
                summary.trim()
            );
            (path, text)
        })
        .collect();
    let root = node("fd-0a0001", "Cookbook");
    let topic = node("fd-0b0001", "Topic");
    let mut files: Vec<(&str, &str)> = vec![
        ("docs/manifestdocs.yaml", "name: cookbook\n"),
        ("docs/cookbook.md", &root),
        ("docs/topic/topic.md", &topic),
    ];
    files.extend(
        children
            .iter()
            .map(|(path, text)| (path.as_str(), text.as_str())),
    );
    let fixture = Fixture::new("context-omitted-list", &files);

    let run = fixture.run("docs", &["context", "fd-0b0001", "--budget", "900"]);

    let (before, size) = estimate(&run);
    let used: u32 = size.split_once(" of ").unwrap().0.parse().unwrap();
    assert!(used <= 810, "{size}\n{before}");
    assert!(before.contains("- Child: C01 "), "{before}");
    assert!(omitted(&run).contains(&"topic/c30/c30.md"), "{before}");
}

#[test]
fn lists_a_linked_doc_it_cant_read() {
    let feature = format!(
        "{}See the [old notes](notes.md).\n",
        node("fd-0a0002", "Feature")
    );
    let fixture = Fixture::new(
        "context-unreadable-doc",
        &[
            ("docs/manifestdocs.yaml", "name: cookbook\n"),
            ("docs/cookbook.md", &node("fd-0a0001", "Cookbook")),
            ("docs/feature/feature.md", &feature),
        ],
    );
    fs::write(fixture.path("docs/feature/notes.md"), b"caf\xe9\n").unwrap();

    let run = fixture.run("docs", &["context", "feature"]);

    assert!(
        run.stdout
            .contains("\n## Other links\n\n- feature/notes.md\n"),
        "{}",
        run.stdout
    );
    assert_eq!(run.code, 0);
}

#[test]
fn shows_the_root_node_once_when_it_is_the_target() {
    let fixture = cookbook("context-root", ENGINE);

    let run = fixture.run("docs", &["context", "."]);

    assert!(
        run.stdout.starts_with(
            "# Context: fd-0a0001 (docs not in git)\n\n## Project: Cookbook (fd-0a0001, draft)\nPath: .\n\n# Cookbook\n\n## Around this feature\n\n- Child: Recipes (fd-0a0002, draft): Recipes summary.\n"
        ),
        "{}",
        run.stdout
    );
    assert_eq!(run.stdout.matches("## Project:").count(), 1);
}

#[test]
fn prints_the_errors_of_a_broken_node_or_stops_on_a_bad_argument() {
    let fixture = cookbook("context-broken", ENGINE);
    let broken = "---\nid: fd-0a0009\ntitle: Broken\nstatus: draft\nsummary: \"Broken.\"\n---\n";
    fs::create_dir_all(fixture.path("docs/broken")).unwrap();
    fs::write(fixture.path("docs/broken/broken.md"), broken).unwrap();

    let node = fixture.run("docs", &["context", "broken"]);
    let missing = fixture.run("docs", &["context", "fd-ffff"]);
    let zero = fixture.run("docs", &["context", "fd-7c2e41", "--budget", "0"]);

    assert_eq!(
        node.stdout,
        "broken/broken.md:3: error: `title` must be double-quoted\n"
    );
    assert_eq!(node.code, 1);
    assert_eq!(missing.stderr, "error: no node `fd-ffff`\n");
    assert_eq!(missing.code, 2);
    assert_eq!(zero.code, 2);
}

#[test]
fn reads_a_decision_id_as_a_path() {
    let fixture = cookbook("context-decision-id", ENGINE);
    fs::create_dir_all(fixture.path("docs/dc-0000002")).unwrap();
    // A folder with no files isn't listed, as in git.
    fs::write(fixture.path("docs/dc-0000002/notes.txt"), "notes\n").unwrap();
    let cases = [
        ("dc-0000001", "error: no file or folder `dc-0000001`\n"),
        ("dc-00", "error: no file or folder `dc-00`\n"),
        (
            "dc-0000002",
            "error: `dc-0000002` isn't a node file or node folder\n",
        ),
    ];
    for (arg, expected) in cases {
        let run = fixture.run("docs", &["context", arg]);

        assert_eq!(run.stderr, expected, "{arg}");
        assert_eq!(run.code, 2, "{arg}");
    }
}

#[test]
fn keeps_ten_percent_of_the_budget_as_headroom() {
    let fixture = cookbook("context-headroom", ENGINE);
    let full = fixture.run("docs", &["context", "fd-7c2e41"]);
    let (_, size) = estimate(&full);
    let tokens: u32 = size.trim_end_matches(" tokens").parse().unwrap();
    // The whole context fits this budget, but not 90% of it.
    let budget = (tokens * 100 / 95).to_string();

    let run = fixture.run("docs", &["context", "fd-7c2e41", "--budget", &budget]);

    assert!(!omitted(&run).is_empty(), "{}", run.stdout);
}

/// Runs `git` in the fixture's repo, with its own home folder so no user config applies.
fn git(fixture: &Fixture, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(["-c", "user.name=Sam", "-c", "user.email=sam@example.com"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(fixture.path(""))
        .env("HOME", fixture.path(""))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_DATE", "2026-10-27T12:00:00Z")
        .env("GIT_COMMITTER_DATE", "2026-10-27T12:00:00Z")
        .output()
        .expect("git runs");
    assert!(output.status.success(), "git {args:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// The context's first line, its header.
fn header(fixture: &Fixture) -> String {
    let run = fixture.run("docs", &["context", "fd-7c2e41"]);
    run.stdout.lines().next().unwrap_or_default().to_owned()
}

#[test]
fn heads_the_context_with_the_docs_branch_commit_and_changes() {
    let fixture = cookbook("context-git", ENGINE);
    git(&fixture, &["init", "-q", "-b", "main"]);
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "Add the cookbook"]);
    let commit = git(&fixture, &["rev-parse", "--short", "HEAD"]);
    let version = |rest: &str| format!("# Context: fd-7c2e41 (docs {rest})");

    let clean = header(&fixture);
    // A change outside the tree that the context doesn't load isn't a docs change.
    fs::write(fixture.path("README.md"), "# Changed\n").unwrap();
    let code_changed = header(&fixture);
    fs::write(fixture.path("adr/engine.md"), "# Engine, edited\n").unwrap();
    let doc_changed = header(&fixture);
    git(&fixture, &["checkout", "-q", "--", "adr/engine.md"]);
    fs::write(fixture.path("docs/glossary.md"), "# Glossary, edited\n").unwrap();
    let tree_changed = header(&fixture);

    assert_eq!(clean, version(&format!("main @ {commit}, 2026-10-27")));
    assert_eq!(code_changed, clean);
    assert_eq!(tree_changed, doc_changed);
    assert_eq!(
        doc_changed,
        version(&format!("main @ {commit}, 2026-10-27, uncommitted changes"))
    );
}

#[test]
fn warns_when_the_docs_clone_is_off_its_default_branch() {
    let fixture = cookbook("context-git-branch", ENGINE);
    git(&fixture, &["init", "-q", "-b", "main"]);
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "Add the cookbook"]);
    let commit = git(&fixture, &["rev-parse", "--short", "HEAD"]);
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

    let on_main = fixture.run("docs", &["context", "fd-7c2e41"]).stdout;
    git(&fixture, &["checkout", "-q", "-b", "feature/filters"]);
    let on_feature = fixture.run("docs", &["context", "fd-7c2e41"]).stdout;
    git(&fixture, &["checkout", "-q", "--detach"]);
    let detached = fixture.run("docs", &["context", "fd-7c2e41"]).stdout;

    assert!(!on_main.contains("Warning:"), "{on_main}");
    assert!(
        on_feature.starts_with(&format!(
            "# Context: fd-7c2e41 (docs feature/filters @ {commit}, 2026-10-27)\n\n\
             Warning: the docs clone is on `feature/filters`, not its default branch `main`.\n"
        )),
        "{on_feature}"
    );
    assert!(
        detached.starts_with(&format!(
            "# Context: fd-7c2e41 (docs detached HEAD @ {commit}, 2026-10-27)\n\n## Feature:"
        )),
        "{detached}"
    );
}

#[test]
fn keeps_each_section_in_order_when_a_linked_doc_cant_be_read() {
    let feature = format!(
        "{}See [bad](bad.md), [good](good.md), [import](../import/import.md) and [a URL](https://example.com/x).\n",
        node("fd-0a0002", "Feature")
    );
    let fixture = Fixture::new(
        "context-unreadable-order",
        &[
            ("docs/manifestdocs.yaml", "name: cookbook\n"),
            ("docs/cookbook.md", &node("fd-0a0001", "Cookbook")),
            ("docs/feature/feature.md", &feature),
            ("docs/feature/good.md", "# Good\n"),
            ("docs/import/import.md", &node("fd-0a0003", "Import")),
        ],
    );
    fs::write(fixture.path("docs/feature/bad.md"), b"caf\xe9\n").unwrap();

    let run = fixture.run("docs", &["context", "feature"]);

    let tail = run.stdout.split_once("\n## Linked docs\n").unwrap().1;
    let (sections, _) = tail.rsplit_once("Estimated size: ").unwrap();
    assert_eq!(
        sections,
        "\n### feature/good.md\n\n# Good\n\n\
         ## Linked nodes\n\n- Import (fd-0a0003, draft): Import summary.\n\n\
         ## Other links\n\n- feature/bad.md\n- https://example.com/x\n\n"
    );
}

#[test]
fn fits_any_budget_that_leaving_everything_out_would_fit() {
    // Short links cost more as omitted entries than kept, which is where trimming has to
    // look past them to the bigger docs before them.
    let urls: String = (1..=16)
        .map(|i| format!("[{i}](//e.example/{i}) "))
        .collect();
    let feature = format!(
        "{}See [mid](mid.md) and [big](big.md). {urls}\n",
        node("fd-0a0002", "Feature")
    );
    let mid = format!("# Mid\n\n{}\n", "m".repeat(300));
    let big = format!("# Big\n\n{}\n", "b".repeat(4_200));
    let fixture = Fixture::new(
        "context-trim-past-links",
        &[
            ("docs/manifestdocs.yaml", "name: cookbook\n"),
            ("docs/cookbook.md", &node("fd-0a0001", "Cookbook")),
            ("docs/feature/feature.md", &feature),
            ("docs/feature/mid.md", &mid),
            ("docs/feature/big.md", &big),
        ],
    );
    let used = |budget: u32| -> u32 {
        let run = fixture.run(
            "docs",
            &["context", "feature", "--budget", &budget.to_string()],
        );
        let (_, size) = estimate(&run);
        size.split_once(" of ").unwrap().0.parse().unwrap()
    };
    // A budget of 1 leaves out everything but the target.
    let smallest = used(1);

    for budget in (100..=900).step_by(10) {
        let room = budget / 10 * 9;
        if smallest <= room {
            assert!(used(budget) <= room, "budget {budget}");
        }
    }
}

/// A decision record on `node`, whose Why paragraph is `why` when it isn't empty.
fn record_with_why(
    id: &str,
    node: &str,
    title: &str,
    status: &str,
    date: &str,
    why: &str,
) -> String {
    let why = match why {
        "" => String::new(),
        why => format!("**Why.** {why}\n\n"),
    };
    format!(
        "---\nid: {id}\nnode: {node}\ntitle: \"{title}\"\nstatus: {status}\ntopic: t-{id}\ndate: {date}\n---\n\n\
         **Decision.** {title}.\n\n{why}**Rejected.** Doing nothing.\n"
    )
}

/// A decision record whose Why paragraph has two sentences, the first naming `title`.
fn record(id: &str, node: &str, title: &str, status: &str, date: &str) -> String {
    let why = format!("{title} helps readers.\nIt also helps agents.");
    record_with_why(id, node, title, status, date, &why)
}

/// `text`, a record, with `supersedes: older` added to its frontmatter.
fn superseding(text: &str, older: &str) -> String {
    text.replacen("\n---\n", &format!("\nsupersedes: {older}\n---\n"), 1)
}

/// A tree in `docs/` with Search under Recipes under the root, and `records` added.
fn decided(name: &str, records: &[(&str, &str)]) -> Fixture {
    let root = node("fd-0a0001", "Cookbook");
    let recipes = node("fd-0a0002", "Recipes");
    let search = node("fd-7c2e41", "Search");
    let mut files = vec![
        ("docs/manifestdocs.yaml", "name: cookbook\n"),
        ("docs/cookbook.md", root.as_str()),
        ("docs/recipes/recipes.md", recipes.as_str()),
        ("docs/recipes/search/search.md", search.as_str()),
    ];
    files.extend_from_slice(records);
    Fixture::new(name, &files)
}

/// The `## Decisions` section, up to the next section or the end of the context.
fn decisions(run: &Run) -> &str {
    let Some((_, tail)) = run.stdout.split_once("\n## Decisions\n") else {
        panic!("no decisions: {}{}", run.stdout, run.stderr);
    };
    let tail = tail.split("\n## ").next().unwrap();
    tail.split("\nEstimated size: ").next().unwrap()
}

#[test]
fn lists_the_accepted_and_proposed_decisions_of_the_node_and_its_ancestors() {
    let github = record(
        "dc-000000a",
        "fd-0a0001",
        "No GitHub writes",
        "accepted",
        "2026-10-01",
    );
    let units = record(
        "dc-000000b",
        "fd-0a0002",
        "Metric units first",
        "proposed",
        "2026-10-02",
    );
    let page = record(
        "dc-000000c",
        "fd-7c2e41",
        "Show 20 per page",
        "accepted",
        "2026-10-03",
    );
    let filters = record(
        "dc-000000d",
        "fd-7c2e41",
        "Filter as picked",
        "proposed",
        "2026-10-04",
    );
    let fixture = decided(
        "context-decisions",
        &[
            ("docs/decisions/2026-10-01-github.md", &github),
            ("docs/recipes/decisions/2026-10-02-units.md", &units),
            ("docs/recipes/search/decisions/2026-10-03-page.md", &page),
            (
                "docs/recipes/search/decisions/2026-10-04-filters.md",
                &filters,
            ),
        ],
    );

    let run = fixture.run("docs", &["context", "fd-7c2e41"]);

    let (before, _) = estimate(&run);
    assert_eq!(
        before,
        "# Context: fd-7c2e41 (docs not in git)\n\n\
         ## Feature: Search (fd-7c2e41, draft)\nPath: recipes/search\n\n# Search\n\n\
         ## Parent: Recipes (fd-0a0002, draft)\nPath: recipes\n\n# Recipes\n\n\
         ## Project: Cookbook (fd-0a0001, draft)\n\n# Cookbook\n\n\
         ## Decisions\n\n\
         - dc-000000d (proposed): Filter as picked. Filter as picked helps readers.\n\
         - dc-000000c (accepted): Show 20 per page. Show 20 per page helps readers.\n\
         - dc-000000b (proposed): Metric units first. Metric units first helps readers.\n\
         - dc-000000a (accepted): No GitHub writes. No GitHub writes helps readers.\n\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn leaves_out_superseded_and_rejected_decisions() {
    let old = record(
        "dc-000000a",
        "fd-7c2e41",
        "Hide empty filters",
        "superseded",
        "2026-10-01",
    );
    let engine = record(
        "dc-000000b",
        "fd-0a0002",
        "Use SQLite FTS",
        "rejected",
        "2026-10-02",
    );
    let fixture = decided(
        "context-decisions-left-out",
        &[
            ("docs/recipes/search/decisions/2026-10-01-empty.md", &old),
            ("docs/recipes/decisions/2026-10-02-engine.md", &engine),
        ],
    );

    let run = fixture.run("docs", &["context", "fd-7c2e41"]);

    assert!(!run.stdout.contains("## Decisions"), "{}", run.stdout);
    assert!(!run.stdout.contains("dc-00000"), "{}", run.stdout);
    assert_eq!(run.code, 0);
}

#[test]
fn ends_a_superseding_decision_with_the_title_of_the_one_it_supersedes() {
    let first = record(
        "dc-0000001",
        "fd-7c2e41",
        "Hide empty filters",
        "superseded",
        "2026-10-01",
    );
    let second = record(
        "dc-0000002",
        "fd-7c2e41",
        "Grey out empty filters",
        "superseded",
        "2026-10-02",
    );
    let third = record(
        "dc-0000003",
        "fd-7c2e41",
        "Count each filter",
        "accepted",
        "2026-10-03",
    );
    let second = superseding(&second, "dc-0000001");
    let third = superseding(&third, "dc-0000002");
    let fixture = decided(
        "context-decisions-supersedes",
        &[
            ("docs/recipes/search/decisions/2026-10-01-empty.md", &first),
            ("docs/recipes/search/decisions/2026-10-02-empty.md", &second),
            ("docs/recipes/search/decisions/2026-10-03-empty.md", &third),
        ],
    );

    let run = fixture.run("docs", &["context", "fd-7c2e41"]);

    assert_eq!(
        decisions(&run),
        "\n- dc-0000003 (accepted): Count each filter. Count each filter helps readers. \
         Supersedes dc-0000002: Grey out empty filters\n"
    );
}

#[test]
fn shows_only_the_title_of_a_decision_with_no_why() {
    let page = record_with_why(
        "dc-000000a",
        "fd-7c2e41",
        "Show 20 per page",
        "accepted",
        "2026-10-01",
        "",
    );
    let fixture = decided(
        "context-decisions-no-why",
        &[("docs/recipes/search/decisions/2026-10-01-page.md", &page)],
    );

    let run = fixture.run("docs", &["context", "fd-7c2e41"]);

    assert_eq!(
        decisions(&run),
        "\n- dc-000000a (accepted): Show 20 per page.\n"
    );
}

#[test]
fn lists_each_nodes_decisions_newest_first() {
    let a = record(
        "dc-000000a",
        "fd-7c2e41",
        "Middle",
        "accepted",
        "2026-10-02",
    );
    let b = record(
        "dc-000000b",
        "fd-7c2e41",
        "Newest",
        "accepted",
        "2026-10-03",
    );
    let c = record(
        "dc-000000c",
        "fd-7c2e41",
        "Oldest",
        "accepted",
        "2026-10-01",
    );
    let fixture = decided(
        "context-decisions-newest-first",
        &[
            ("docs/recipes/search/decisions/a.md", &a),
            ("docs/recipes/search/decisions/b.md", &b),
            ("docs/recipes/search/decisions/c.md", &c),
        ],
    );

    let run = fixture.run("docs", &["context", "fd-7c2e41"]);

    let ids: Vec<&str> = decisions(&run)
        .lines()
        .filter_map(|line| line.strip_prefix("- "))
        .map(|line| line.split(' ').next().unwrap())
        .collect();
    assert_eq!(ids, ["dc-000000b", "dc-000000a", "dc-000000c"]);
}

#[test]
fn trims_the_oldest_decisions_of_the_farthest_ancestor_first() {
    // One long sentence, so a decision's line costs more than its entry in the omitted list.
    let why = format!("{}.", "readers need it ".repeat(10).trim_end());
    let records: Vec<(String, String)> = [
        ("docs/decisions", "fd-0a0001", "a"),
        ("docs/recipes/decisions", "fd-0a0002", "b"),
        ("docs/recipes/search/decisions", "fd-7c2e41", "c"),
    ]
    .iter()
    .flat_map(|(folder, node, letter)| {
        [("1", "2026-10-01", "old"), ("2", "2026-10-02", "new")].map(|(n, date, name)| {
            let id = format!("dc-00000{letter}{n}");
            let text = record_with_why(&id, node, name, "accepted", date, &why);
            (format!("{folder}/{date}-{name}.md"), text)
        })
    })
    .collect();
    let records: Vec<(&str, &str)> = records
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();
    let fixture = decided("context-decisions-budget", &records);
    let full = fixture.run("docs", &["context", "fd-7c2e41"]);
    let tokens: u32 = estimate(&full)
        .1
        .trim_end_matches(" tokens")
        .parse()
        .unwrap();

    // Going down from a budget that fits everything, the contexts in which one and then two
    // pieces are first left out.
    let mut first: Vec<(Vec<String>, String)> = Vec::new();
    for budget in (1..=tokens * 2).rev().step_by(5) {
        let run = fixture.run(
            "docs",
            &["context", "fd-7c2e41", "--budget", &budget.to_string()],
        );
        let names: Vec<String> = omitted(&run).iter().map(|name| name.to_string()).collect();
        if names.len() > first.len() {
            first.push((names, run.stdout.clone()));
        }
        if first.len() == 2 {
            break;
        }
    }

    assert_eq!(first[0].0, ["decisions/2026-10-01-old.md"]);
    assert_eq!(
        first[1].0,
        ["decisions/2026-10-02-new.md", "decisions/2026-10-01-old.md"]
    );
    for id in ["dc-00000b1", "dc-00000b2", "dc-00000c1", "dc-00000c2"] {
        assert!(
            first[1].1.contains(&format!("- {id} ")),
            "{id}: {}",
            first[1].1
        );
    }
}

/// From another repo, records come from `origin/HEAD`, like the rest of the context.
#[test]
fn reads_decisions_from_origin_head_through_a_pointer() {
    let (fixture, _, _) = workspace("context-decisions-origin-head");
    let write = |file: &str, id: &str, title: &str| {
        let text = record(id, "fd-7c2e41", title, "accepted", "2026-10-01");
        let path = fixture.path(&format!("work/docs/search/decisions/{file}"));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    };
    let docs = "work/docs";
    write("2026-10-01-merged.md", "dc-000000a", "Merged");
    docs_git(&fixture, docs, &["add", "-A"]);
    docs_git(&fixture, docs, &["commit", "-q", "-m", "Record a decision"]);
    docs_git(&fixture, docs, &["push", "-q", "origin", "feature/x:main"]);
    docs_git(&fixture, docs, &["fetch", "-q", "origin"]);
    write("2026-10-02-draft.md", "dc-000000b", "Uncommitted");

    let run = fixture.run("work/web", &["context", "fd-7c2e41"]);

    assert_eq!(
        decisions(&run),
        "\n- dc-000000a (accepted): Merged. Merged helps readers.\n"
    );
    assert_eq!(run.code, 0, "{}", run.stderr);
}

#[test]
fn ref_reads_the_decisions_on_another_branch() {
    let (fixture, _, _) = workspace("context-decisions-ref");
    let text = record(
        "dc-000000a",
        "fd-7c2e41",
        "Branch",
        "accepted",
        "2026-10-01",
    );
    let path = fixture.path("work/docs/search/decisions/2026-10-01-branch.md");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
    let docs = "work/docs";
    docs_git(&fixture, docs, &["add", "-A"]);
    docs_git(&fixture, docs, &["commit", "-q", "-m", "Record a decision"]);

    let on_branch = fixture.run("work/web", &["context", "fd-7c2e41", "--ref", "feature/x"]);
    let merged = fixture.run("work/web", &["context", "fd-7c2e41"]);

    assert_eq!(
        decisions(&on_branch),
        "\n- dc-000000a (accepted): Branch. Branch helps readers.\n"
    );
    assert!(!merged.stdout.contains("## Decisions"), "{}", merged.stdout);
}

#[test]
fn shows_the_root_nodes_decisions_when_it_is_the_target() {
    let github = record(
        "dc-000000a",
        "fd-0a0001",
        "No GitHub writes",
        "accepted",
        "2026-10-01",
    );
    let units = record(
        "dc-000000b",
        "fd-0a0002",
        "Metric units first",
        "accepted",
        "2026-10-02",
    );
    let fixture = decided(
        "context-decisions-root",
        &[
            ("docs/decisions/2026-10-01-github.md", &github),
            ("docs/recipes/decisions/2026-10-02-units.md", &units),
        ],
    );

    let run = fixture.run("docs", &["context", "."]);

    assert_eq!(
        decisions(&run),
        "\n- dc-000000a (accepted): No GitHub writes. No GitHub writes helps readers.\n"
    );
}

#[test]
fn leaves_out_the_decisions_of_children_and_siblings() {
    let filters = node("fd-7c2e99", "Filters");
    let import = node("fd-0a0003", "Import");
    let page = record(
        "dc-000000a",
        "fd-7c2e41",
        "Show 20 per page",
        "accepted",
        "2026-10-01",
    );
    let child = record(
        "dc-000000b",
        "fd-7c2e99",
        "Filter as picked",
        "accepted",
        "2026-10-02",
    );
    let sibling = record(
        "dc-000000c",
        "fd-0a0003",
        "Import CSV first",
        "accepted",
        "2026-10-03",
    );
    let fixture = decided(
        "context-decisions-relatives",
        &[
            ("docs/recipes/search/filters/filters.md", &filters),
            ("docs/recipes/import/import.md", &import),
            ("docs/recipes/search/decisions/2026-10-01-page.md", &page),
            (
                "docs/recipes/search/filters/decisions/2026-10-02-picked.md",
                &child,
            ),
            ("docs/recipes/import/decisions/2026-10-03-csv.md", &sibling),
        ],
    );

    let run = fixture.run("docs", &["context", "fd-7c2e41"]);

    assert_eq!(
        decisions(&run),
        "\n- dc-000000a (accepted): Show 20 per page. Show 20 per page helps readers.\n"
    );
}

#[test]
fn ends_with_only_the_id_when_the_superseded_record_is_missing_or_broken() {
    let import = node("fd-0a0003", "Import");
    let missing = record(
        "dc-000000a",
        "fd-7c2e41",
        "Count each filter",
        "accepted",
        "2026-10-01",
    );
    let missing = superseding(&missing, "dc-0000009");
    let broken = record(
        "dc-000000b",
        "fd-7c2e41",
        "Show 20 per page",
        "accepted",
        "2026-10-02",
    );
    let broken = superseding(&broken, "dc-000000e");
    let older = "---\nid: dc-000000e\nnode: fd-0a0003\nstatus: superseded\n---\n";
    let fixture = decided(
        "context-decisions-missing-predecessor",
        &[
            ("docs/recipes/import/import.md", &import),
            (
                "docs/recipes/search/decisions/2026-10-01-count.md",
                &missing,
            ),
            ("docs/recipes/search/decisions/2026-10-02-page.md", &broken),
            ("docs/recipes/import/decisions/2026-09-01-old.md", older),
        ],
    );

    let run = fixture.run("docs", &["context", "fd-7c2e41"]);

    assert_eq!(
        decisions(&run),
        "\n- dc-000000b (accepted): Show 20 per page. Show 20 per page helps readers. Supersedes dc-000000e\n\
         - dc-000000a (accepted): Count each filter. Count each filter helps readers. Supersedes dc-0000009\n"
    );
}

#[test]
fn reads_a_superseded_record_from_another_node() {
    let import = node("fd-0a0003", "Import");
    let older = record(
        "dc-000000f",
        "fd-0a0003",
        "Old import rule",
        "superseded",
        "2026-09-01",
    );
    let newer = record(
        "dc-000000a",
        "fd-7c2e41",
        "New rule",
        "accepted",
        "2026-10-01",
    );
    let newer = superseding(&newer, "dc-000000f");
    let fixture = decided(
        "context-decisions-predecessor-elsewhere",
        &[
            ("docs/recipes/import/import.md", &import),
            ("docs/recipes/import/decisions/2026-09-01-old.md", &older),
            ("docs/recipes/search/decisions/2026-10-01-new.md", &newer),
        ],
    );

    let run = fixture.run("docs", &["context", "fd-7c2e41"]);

    assert_eq!(
        decisions(&run),
        "\n- dc-000000a (accepted): New rule. New rule helps readers. Supersedes dc-000000f: Old import rule\n"
    );
}

#[test]
fn lists_a_record_with_frontmatter_errors_by_its_id_or_path() {
    let page = record(
        "dc-000000a",
        "fd-7c2e41",
        "Show 20 per page",
        "accepted",
        "2026-10-01",
    );
    let broken = "---\nid: dc-000000e\nnode: fd-7c2e41\nstatus: accepted\n---\n";
    let no_id = "---\nnode: fd-7c2e41\ntitle: \"No ID\"\n---\n";
    let fixture = decided(
        "context-decisions-broken-record",
        &[
            ("docs/recipes/search/decisions/2026-10-01-page.md", &page),
            ("docs/recipes/search/decisions/2026-10-02-broken.md", broken),
            ("docs/recipes/search/decisions/2026-10-03-no-id.md", no_id),
        ],
    );

    let run = fixture.run("docs", &["context", "fd-7c2e41"]);

    assert_eq!(
        decisions(&run),
        "\n- dc-000000a (accepted): Show 20 per page. Show 20 per page helps readers.\n\
         - dc-000000e (has problems): recipes/search/decisions/2026-10-02-broken.md\n\
         - recipes/search/decisions/2026-10-03-no-id.md (has problems)\n"
    );
    assert_eq!(run.code, 0);
}

#[test]
fn lists_the_decisions_of_an_ancestor_with_frontmatter_errors() {
    let units = record(
        "dc-000000b",
        "fd-0a0002",
        "Metric units first",
        "accepted",
        "2026-10-02",
    );
    let fixture = decided(
        "context-decisions-broken-ancestor",
        &[("docs/recipes/decisions/2026-10-02-units.md", &units)],
    );
    let broken = "---\nid: fd-0a0002\ntitle: Recipes\nstatus: draft\nsummary: \"Recipes.\"\n---\n";
    fs::write(fixture.path("docs/recipes/recipes.md"), broken).unwrap();

    let run = fixture.run("docs", &["context", "fd-7c2e41"]);

    assert!(
        run.stdout
            .contains("## Parent: recipes (fd-0a0002, has problems)"),
        "{}",
        run.stdout
    );
    assert_eq!(
        decisions(&run),
        "\n- dc-000000b (accepted): Metric units first. Metric units first helps readers.\n"
    );
}

#[test]
fn skips_a_record_file_it_cant_read() {
    let import = node("fd-0a0003", "Import");
    let page = record(
        "dc-000000a",
        "fd-7c2e41",
        "Show 20 per page",
        "accepted",
        "2026-10-01",
    );
    let fixture = decided(
        "context-decisions-unreadable",
        &[
            ("docs/recipes/import/import.md", &import),
            ("docs/recipes/search/decisions/2026-10-01-page.md", &page),
        ],
    );
    for folder in [
        "docs/recipes/search/decisions",
        "docs/recipes/import/decisions",
    ] {
        fs::create_dir_all(fixture.path(folder)).unwrap();
        fs::write(
            fixture.path(&format!("{folder}/2026-10-02-bad.md")),
            b"caf\xe9\n",
        )
        .unwrap();
    }

    let run = fixture.run("docs", &["context", "fd-7c2e41"]);

    assert_eq!(
        decisions(&run),
        "\n- dc-000000a (accepted): Show 20 per page. Show 20 per page helps readers.\n"
    );
    assert_eq!(run.code, 0, "{}", run.stderr);
}
