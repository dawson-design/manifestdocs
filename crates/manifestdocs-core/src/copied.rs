//! `check`'s optional rule for issues that copy their node's text instead of linking to it.

use std::collections::BTreeSet;

use pulldown_cmark::{Event, Parser};

use crate::finding::{Finding, Problem};
use crate::id::NodeId;
use crate::node::ParsedNode;
use crate::paths::TreePath;
use crate::work::{GitHubAnswers, IssueBody, IssueKey, IssueLinks, WorkLinks};

/// The fewest consecutive words an issue shares with its node's body for `check` to warn.
pub const COPIED_WORDS: usize = 50;

/// A node's body, which the rule compares the node's linked issues with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeBody {
    pub id: NodeId,
    pub file: TreePath,
    pub text: String,
}

/// The body of each node whose frontmatter parses.
pub fn node_bodies(nodes: &[ParsedNode]) -> Vec<NodeBody> {
    let parsed = nodes
        .iter()
        .filter_map(|p| p.node.as_ref().map(|node| (p, node)));
    parsed
        .map(|(parsed, node)| NodeBody {
            id: node.id.clone(),
            file: parsed.file.clone(),
            text: node.body.clone(),
        })
        .collect()
}

/// A warning on a node's file for each issue linked to the node, through its `work` or the
/// issue's `feature:` line, whose body copies `COPIED_WORDS` or more words in a row from the
/// node's body.
pub fn copied_spec_findings(links: &WorkLinks, answers: &GitHubAnswers) -> Vec<Finding> {
    let mut seen = BTreeSet::new();
    let pairs = linked(links, answers)
        .filter(|(node, issue)| seen.insert((node.file.clone(), IssueKey::from(&issue.issue))));
    pairs
        .filter_map(|(node, issue)| {
            let words = longest_shared_run(&issue.body, &node.text);
            let issue = issue.issue.clone();
            (words >= COPIED_WORDS).then(|| Finding {
                file: node.file.clone(),
                line: None,
                problem: Problem::IssueCopiesSpec { issue, words },
            })
        })
        .collect()
}

/// Each issue body with a node it's linked to.
fn linked<'a>(
    links: &'a WorkLinks,
    answers: &'a GitHubAnswers,
) -> impl Iterator<Item = (&'a NodeBody, &'a IssueBody)> {
    let listed = answers.work_bodies.iter().flat_map(move |issue| {
        let items = links.work.iter().filter(|item| item.issue == issue.issue);
        let nodes = items.filter_map(|item| links.bodies.iter().find(|n| n.file == item.file));
        nodes.map(move |node| (node, issue))
    });
    let named = answers.searched.iter().flat_map(move |issue| {
        let id = IssueLinks::parse(&issue.body).feature;
        let nodes = links
            .bodies
            .iter()
            .filter(move |node| id.as_ref() == Some(&node.id));
        nodes.map(move |node| (node, issue))
    });
    listed.chain(named)
}

/// The most consecutive words `issue` shares with `spec`, both read as the text their Markdown
/// renders, so a link counts by its visible text. A word is a run of letters and digits, with
/// any apostrophes inside it, compared without case.
pub fn longest_shared_run(issue: &str, spec: &str) -> usize {
    let (issue, spec) = (words(issue), words(spec));
    // `run[j + 1]`: the shared run ending at the current issue word and `spec[j]`. Going
    // down through `j` leaves `run[j]` holding the previous issue word's value.
    let mut run = vec![0; spec.len() + 1];
    let mut longest = 0;
    for word in &issue {
        for j in (0..spec.len()).rev() {
            run[j + 1] = if spec[j] == *word { run[j] + 1 } else { 0 };
            longest = longest.max(run[j + 1]);
        }
    }
    longest
}

fn words(markdown: &str) -> Vec<String> {
    let text = visible_text(markdown).to_lowercase().replace('’', "'");
    let words = text.split(|c: char| !(c.is_alphanumeric() || c == '\''));
    let words = words.map(|word| word.trim_matches('\''));
    words
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The text `markdown` renders as, without link destinations or HTML. Every tag and line
/// break adds a space, so text in separate blocks or cells doesn't run together.
fn visible_text(markdown: &str) -> String {
    let mut text = String::new();
    for event in Parser::new(markdown) {
        match event {
            Event::Text(part) | Event::Code(part) => text.push_str(&part),
            Event::Html(_) | Event::InlineHtml(_) => {}
            _ => text.push(' '),
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_the_longest_run_of_shared_words() {
        let spec = "one two three four five six seven";
        assert_eq!(longest_shared_run("two three nine five six seven", spec), 3);
        assert_eq!(longest_shared_run("zero one two three four zero", spec), 4);
        assert_eq!(longest_shared_run(spec, spec), 7);
    }

    #[test]
    fn ignores_case_punctuation_markdown_and_line_breaks() {
        let spec = "Search finds recipes by [ingredient](../rules/rules.md), cuisine and `time`.";
        let issue = "- **search** finds\r\nRECIPES by ingredient; cuisine <br> and time";
        assert_eq!(longest_shared_run(issue, spec), 8);
    }

    #[test]
    fn reads_links_and_images_by_their_visible_text() {
        let spec =
            "a ![photo](img/photo.png) of <https://example.com> and [rules][r] here\n\n[r]: x.md";
        let issue = "a photo of https://example.com and rules here";
        assert_eq!(longest_shared_run(issue, spec), 9);
    }

    #[test]
    fn keeps_an_apostrophe_inside_its_word() {
        let spec = "the recipe's title and 'its' photo";
        assert_eq!(
            longest_shared_run("The recipe’s title and its photo", spec),
            6
        );
        assert_eq!(longest_shared_run("recipe s title", spec), 1);
    }

    #[test]
    fn shares_nothing_with_empty_text() {
        assert_eq!(longest_shared_run("", "some spec text"), 0);
        assert_eq!(longest_shared_run("some issue text", ""), 0);
        assert_eq!(longest_shared_run("alpha beta", "gamma delta"), 0);
    }
}
