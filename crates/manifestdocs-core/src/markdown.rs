//! One pass over a Markdown body for its links, images and headings.

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outline {
    pub links: Vec<Link>,
    /// Each heading's text, as GitHub builds its anchor from.
    pub headings: Vec<String>,
    /// `id` and `name` values in raw HTML, which are anchors too.
    pub html_anchors: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    /// The destination as written, or `mailto:` and the address for an email autolink.
    pub target: String,
    pub line: u32,
}

/// Reads `body`, whose first line is `first_line` of its file. Code spans and blocks hold no
/// links or headings.
pub fn outline(body: &str, first_line: u32) -> Outline {
    let line_starts = line_starts(body);
    let mut outline = Outline::default();
    let mut heading: Option<String> = None;
    for (event, range) in Parser::new_ext(body, options()).into_offset_iter() {
        let line = first_line + line_index(&line_starts, range.start);
        match event {
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                ..
            })
            | Event::Start(Tag::Image {
                link_type,
                dest_url,
                ..
            }) => outline.links.push(Link {
                target: target(link_type, &dest_url),
                line,
            }),
            Event::Start(Tag::Heading { .. }) => heading = Some(String::new()),
            Event::End(TagEnd::Heading(_)) => outline.headings.extend(heading.take()),
            Event::Text(text) | Event::Code(text) => {
                heading.iter_mut().for_each(|h| h.push_str(&text));
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                outline.html_anchors.extend(html_anchors(&html));
            }
            _ => {}
        }
    }
    outline
}

fn html_anchors(html: &str) -> Vec<String> {
    let starts = ["id=\"", "name=\""].into_iter().flat_map(|attribute| {
        let found = html.match_indices(attribute);
        found.map(move |(start, _)| start + attribute.len())
    });
    let values = starts.filter_map(|start| html[start..].split_once('"'));
    values.map(|(value, _)| value.to_owned()).collect()
}

// GitHub renders tables, footnotes, strikethrough and task lists, which change what counts
// as a link or a heading.
fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
}

fn target(link_type: LinkType, dest_url: &str) -> String {
    match link_type {
        LinkType::Email => format!("mailto:{dest_url}"),
        _ => dest_url.to_owned(),
    }
}

fn line_starts(text: &str) -> Vec<usize> {
    let after_newlines = text.match_indices('\n').map(|(index, _)| index + 1);
    std::iter::once(0).chain(after_newlines).collect()
}

fn line_index(line_starts: &[usize], offset: usize) -> u32 {
    let index = line_starts.partition_point(|start| *start <= offset) - 1;
    u32::try_from(index).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn links(body: &str) -> Vec<(String, u32)> {
        let links = outline(body, 10).links.into_iter();
        links.map(|link| (link.target, link.line)).collect()
    }

    #[test]
    fn finds_inline_reference_image_and_autolinks_with_their_lines() {
        let body = "[a](a.md)\n\n![b](b.png)\n\n[c][ref]\n\n<https://example.com>\n<sam@example.com>\n\n[ref]: c.md\n";

        assert_eq!(
            links(body),
            [
                ("a.md".into(), 10),
                ("b.png".into(), 12),
                ("c.md".into(), 14),
                ("https://example.com".into(), 16),
                ("mailto:sam@example.com".into(), 17),
            ]
        );
    }

    #[test]
    fn skips_code_unused_definitions_and_missing_references() {
        let body = "`[a](a.md)`\n\n```\n[b](b.md)\n# Not a heading\n```\n\n    [c](c.md)\n\n[d][missing]\n\n[unused]: e.md\n";

        assert_eq!(links(body), []);
        assert_eq!(outline(body, 1).headings, Vec::<String>::new());
    }

    #[test]
    fn collects_html_id_and_name_values_as_anchors() {
        let body = "<a id=\"step-one\"></a>\n\nText with <span name=\"two\">x</span>.\n";

        assert_eq!(outline(body, 1).html_anchors, ["step-one", "two"]);
    }

    #[test]
    fn reads_heading_text_including_code_and_setext_headings() {
        let body = "# Write `path`\n\nSetext heading\n---\n\n## A & B\n";

        assert_eq!(
            outline(body, 1).headings,
            ["Write path", "Setext heading", "A & B"]
        );
    }
}
