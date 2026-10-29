use crate::support::{Fixture, node};

const ON: &str = "name: demo\ngithub:\n  host: github.com\n  docs_repo: acme/cookbook-docs\n  check_copied_spec: true\n";
const OFF: &str = "name: demo\ngithub:\n  host: github.com\n  docs_repo: acme/cookbook-docs\n";

/// The body of `search/search.md`: 68 words.
const SPEC: &str = "Search finds recipes by ingredient, cuisine and cooking time. Results are ranked by how many of the given ingredients a recipe uses, then by rating, then by how recently the recipe was added. A search with no matches suggests removing the least common ingredient. Each result shows the recipe's title, its photo, the total cooking time and the ingredients it is missing, so a cook can decide quickly.";

/// The first `n` words of `SPEC`, as an issue might paste them.
fn copied(n: usize) -> String {
    let words: Vec<&str> = SPEC.split_whitespace().take(n).collect();
    words.join(" ")
}

/// A node file whose `work` lists `work`, with `body` below a heading.
fn spec_node(id: &str, title: &str, work: &str, body: &str) -> String {
    format!(
        "---\nid: {id}\ntitle: \"{title}\"\nstatus: draft\nsummary: \"{title}.\"\nwork: [{work}]\n---\n\n# {title}\n\n{body}\n"
    )
}

/// A tree with a root node and `search/search.md` (`fd-000002`), whose `work` lists `work`
/// and whose body is `SPEC`.
fn tree(name: &str, config: &str, work: &str) -> Fixture {
    tree_with_body(name, config, work, SPEC)
}

fn tree_with_body(name: &str, config: &str, work: &str, body: &str) -> Fixture {
    Fixture::new(
        name,
        &[
            ("manifestdocs.yaml", config),
            ("demo.md", &node("fd-000001", "Demo")),
            (
                "search/search.md",
                &spec_node("fd-000002", "Search", work, body),
            ),
        ],
    )
}

fn warning(issue: &str, words: usize) -> String {
    format!(
        "search/search.md: warning: issue `{issue}` copies {words} words in a row from this node; link to the node instead\n\n2 nodes: 1 warning\n"
    )
}

#[test]
fn warns_about_an_issue_that_copies_50_words_of_the_node_it_names() {
    let fixture = tree("copied-feature", ON, "");
    let body = format!(
        "feature: fd-000002\n\n## Requirements\n\n- {}\n",
        copied(50)
    );
    fixture.gh_search("github.com", "acme/cookbook-docs", &[(7, &body)]);

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(run.stdout, warning("acme/cookbook-docs#7", 50));
    assert_eq!(run.stderr, "");
    assert_eq!(run.code, 0);
}

#[test]
fn warns_about_a_work_item_that_copies_its_nodes_text() {
    let fixture = tree("copied-work", ON, "acme/cookbook-docs#41");
    let body = format!("SEARCH FINDS RECIPES\n{}", copied(68).to_lowercase());
    fixture.gh_issue_with_body("github.com", "acme/cookbook-docs#41", &body);
    fixture.gh_search("github.com", "acme/cookbook-docs", &[]);

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(run.stdout, warning("acme/cookbook-docs#41", 68));
    let lookups: Vec<String> = fixture
        .gh_calls()
        .into_iter()
        .filter(|call| call.contains("repository("))
        .collect();
    assert_eq!(lookups.len(), 1);
    assert!(lookups[0].contains("i41: issue(number: 41) { number body }"));
}

#[test]
fn passes_an_issue_that_quotes_fewer_than_50_words() {
    let fixture = tree("copied-short", ON, "");
    let body = format!("feature: fd-000002\n\n> {}\n", copied(49));
    fixture.gh_search("github.com", "acme/cookbook-docs", &[(7, &body)]);

    let run = fixture.run_with_gh("", &["check", "--strict"]);

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(run.code, 0);
}

#[test]
fn warns_once_about_an_issue_linked_both_ways() {
    let fixture = tree("copied-twice", ON, "acme/cookbook-docs#7");
    let body = format!("feature: fd-000002\n\n{}\n", copied(60));
    fixture.gh_issue_with_body("github.com", "acme/cookbook-docs#7", &body);
    fixture.gh_search("github.com", "acme/cookbook-docs", &[(7, &body)]);

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(run.stdout, warning("acme/cookbook-docs#7", 60));
}

#[test]
fn leaves_copied_text_alone_unless_switched_on() {
    let fixture = tree("copied-off", OFF, "acme/cookbook-docs#41");
    fixture.gh_issue_with_body("github.com", "acme/cookbook-docs#41", &copied(68));
    let body = format!("feature: fd-000002\n\n{}\n", copied(68));
    fixture.gh_search("github.com", "acme/cookbook-docs", &[(7, &body)]);

    let run = fixture.run_with_gh("", &["check", "--strict"]);

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    let calls = fixture.gh_calls();
    let lookup = calls.iter().find(|call| call.contains("repository("));
    assert!(
        lookup.is_some_and(|call| call.contains("{ number } }")),
        "{calls:?}"
    );
}

#[test]
fn offline_skips_the_copied_text_rule() {
    let fixture = tree("copied-offline", ON, "acme/cookbook-docs#41");
    fixture.gh_issue_with_body("github.com", "acme/cookbook-docs#41", &copied(68));

    let run = fixture.run_with_gh("", &["check", "--offline", "--strict"]);

    assert_eq!(run.stdout, "2 nodes: no problems\n");
    assert_eq!(fixture.gh_calls(), Vec::<String>::new());
}

#[test]
fn reads_a_link_in_the_nodes_text_as_its_visible_text() {
    let body = SPEC.replace("by rating,", "by [rating](../demo.md#demo),");
    let fixture = tree_with_body("copied-link", ON, "", &body);
    let issue = format!("feature: fd-000002\n\n{}\n", copied(68));
    fixture.gh_search("github.com", "acme/cookbook-docs", &[(7, &issue)]);

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(run.stdout, warning("acme/cookbook-docs#7", 68));
}

#[test]
fn warns_on_each_node_an_issue_copies_from() {
    let fixture = tree("copied-two-nodes", ON, "acme/cookbook-docs#7");
    let ranking = spec_node("fd-000003", "Ranking", "", SPEC);
    std::fs::create_dir_all(fixture.path("ranking")).expect("create a folder");
    std::fs::write(fixture.path("ranking/ranking.md"), ranking).expect("write a node");
    let body = format!("feature: fd-000003\n\n{}\n", copied(60));
    fixture.gh_issue_with_body("github.com", "acme/cookbook-docs#7", &body);
    fixture.gh_search("github.com", "acme/cookbook-docs", &[(7, &body)]);

    let run = fixture.run_with_gh("", &["check"]);

    let message = "warning: issue `acme/cookbook-docs#7` copies 60 words in a row from this node; link to the node instead";
    assert_eq!(
        run.stdout,
        format!(
            "ranking/ranking.md: {message}\nsearch/search.md: {message}\n\n3 nodes: 2 warnings\n"
        )
    );
}

#[test]
fn only_warns_about_the_unknown_node_a_copying_issue_names() {
    let fixture = tree("copied-unknown-node", ON, "");
    let body = format!("feature: fd-abcdef\n\n{}\n", copied(68));
    fixture.gh_search("github.com", "acme/cookbook-docs", &[(7, &body)]);

    let run = fixture.run_with_gh("", &["check"]);

    assert_eq!(
        run.stdout,
        "manifestdocs.yaml: warning: issue `acme/cookbook-docs#7` has `feature: fd-abcdef`, which isn't a node in this tree\n\n2 nodes: 1 warning\n"
    );
}
