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
    // GitHub's anchor leaves out the alt text of an image in a heading.
    let mut image_depth = 0;
    for (event, range) in Parser::new_ext(body, options()).into_offset_iter() {
        let line = first_line + line_index(&line_starts, range.start);
        match event {
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                ..
            }) => {
                outline.links.push(link(link_type, &dest_url, line));
            }
            Event::Start(Tag::Image {
                link_type,
                dest_url,
                ..
            }) => {
                image_depth += 1;
                outline.links.push(link(link_type, &dest_url, line));
            }
            Event::End(TagEnd::Image) => image_depth -= 1,
            Event::Start(Tag::Heading { .. }) => heading = Some(String::new()),
            Event::End(TagEnd::Heading(_)) => outline.headings.extend(heading.take()),
            Event::Text(text) | Event::Code(text) if image_depth == 0 => {
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

// `id` and `name` attributes in any case, with or without quotes, outside comments.
fn html_anchors(html: &str) -> Vec<String> {
    if html.trim_start().starts_with("<!--") {
        return Vec::new();
    }
    let lower = html.to_ascii_lowercase();
    let starts: Vec<usize> = ["id", "name"]
        .into_iter()
        .flat_map(|name| {
            lower
                .match_indices(name)
                .map(move |(at, _)| (at, name.len()))
        })
        .filter(|(at, _)| lower[..*at].ends_with(char::is_whitespace))
        .map(|(at, length)| at + length)
        .collect();
    starts
        .into_iter()
        .filter_map(|start| attribute_value(&html[start..]))
        .collect()
}

fn attribute_value(rest: &str) -> Option<String> {
    let rest = rest.trim_start().strip_prefix('=')?.trim_start();
    let value = match rest.chars().next()? {
        quote @ ('"' | '\'') => rest[1..].split(quote).next()?,
        _ => rest
            .split(|c: char| c.is_whitespace() || matches!(c, '>' | '/'))
            .next()?,
    };
    Some(value.to_owned())
}

// GitHub renders tables, footnotes, strikethrough and task lists, which change what counts
// as a link or a heading.
fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
}

fn link(link_type: LinkType, dest_url: &str, line: u32) -> Link {
    let target = match link_type {
        LinkType::Email => format!("mailto:{dest_url}"),
        _ => dest_url.to_owned(),
    };
    Link { target, line }
}

fn line_starts(text: &str) -> Vec<usize> {
    let after_newlines = text.match_indices('\n').map(|(index, _)| index + 1);
    std::iter::once(0).chain(after_newlines).collect()
}

fn line_index(line_starts: &[usize], offset: usize) -> u32 {
    let index = line_starts.partition_point(|start| *start <= offset) - 1;
    u32::try_from(index).unwrap_or(u32::MAX)
}
