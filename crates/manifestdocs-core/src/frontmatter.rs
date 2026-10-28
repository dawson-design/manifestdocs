//! The YAML block between `---` lines at the top of a Markdown file.

/// The YAML block always starts on the file's second line.
pub const FIRST_YAML_LINE: u32 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Split<'a> {
    /// The file has no frontmatter.
    Absent,
    /// The file opens a block with `---` but never closes it.
    Unclosed,
    Present(Frontmatter<'a>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frontmatter<'a> {
    /// The YAML text, without the `---` lines.
    pub yaml: &'a str,
    pub body: &'a str,
    /// The top-level keys, read from the text so they're known even when the YAML is broken.
    pub keys: Vec<Key>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Key {
    pub name: String,
    /// The file line, counting the opening `---` as line 1.
    pub line: u32,
    /// The text after the colon, trimmed, before any YAML parsing.
    pub value: String,
    pub double_quoted: bool,
}

pub fn split(text: &str) -> Split<'_> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.split_inclusive('\n');
    match lines.next() {
        Some(first) if is_fence(first) => {}
        _ => return Split::Absent,
    }
    let start = text.find('\n').map_or(text.len(), |end| end + 1);
    let mut offset = start;
    for line in lines {
        if is_fence(line) {
            let yaml = &text[start..offset];
            return Split::Present(Frontmatter {
                yaml,
                body: &text[offset + line.len()..],
                keys: keys(yaml),
            });
        }
        offset += line.len();
    }
    Split::Unclosed
}

impl Frontmatter<'_> {
    pub fn key(&self, name: &str) -> Option<&Key> {
        self.keys.iter().find(|key| key.name == name)
    }
}

fn is_fence(line: &str) -> bool {
    line.trim_end() == "---"
}

fn keys(yaml: &str) -> Vec<Key> {
    let lines = (FIRST_YAML_LINE..).zip(yaml.lines());
    lines.filter_map(|(line, text)| key(line, text)).collect()
}

// A top-level key starts the line, so indented, list and comment lines are skipped.
fn key(line: u32, text: &str) -> Option<Key> {
    let (name, value) = text.split_once(':')?;
    let starts_well = name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_');
    let name_chars = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'));
    let ends_key = value.is_empty() || value.starts_with([' ', '\t']);
    (starts_well && name_chars && ends_key).then(|| Key {
        name: name.to_owned(),
        line,
        value: value.trim().to_owned(),
        double_quoted: value.trim_start().starts_with('"'),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn present(text: &str) -> Frontmatter<'_> {
        match split(text) {
            Split::Present(frontmatter) => frontmatter,
            other => panic!("expected frontmatter in {text:?}, got {other:?}"),
        }
    }

    #[test]
    fn splits_the_yaml_from_the_body() {
        let frontmatter = present("---\nid: fd-000001\n---\n\n# Title\n");

        assert_eq!(frontmatter.yaml, "id: fd-000001\n");
        assert_eq!(frontmatter.body, "\n# Title\n");
    }

    #[test]
    fn finds_no_frontmatter_unless_the_first_line_opens_it() {
        assert_eq!(split("# Title\n"), Split::Absent);
        assert_eq!(split("\n---\nid: x\n---\n"), Split::Absent);
        assert_eq!(split(""), Split::Absent);
    }

    #[test]
    fn reports_a_block_that_never_closes() {
        assert_eq!(split("---\nid: fd-000001\n# Title\n"), Split::Unclosed);
    }

    #[test]
    fn accepts_crlf_line_endings_a_byte_order_mark_and_an_empty_block() {
        assert_eq!(present("---\r\nid: x\r\n---\r\nbody").yaml, "id: x\r\n");
        assert_eq!(present("\u{feff}---\nid: x\n---\n").yaml, "id: x\n");
        assert_eq!(present("---\n---\nbody").yaml, "");
    }

    #[test]
    fn scans_top_level_keys_with_their_lines_and_quoting() {
        let text = "---\nid: fd-000001\ntitle: \"Search\"\nsummary: Finds: recipes\n# note: no\nrepos:\n  - acme/web\nwork: [a]\n---\n";
        let keys = present(text).keys;
        let scanned: Vec<(&str, u32, bool)> = keys
            .iter()
            .map(|key| (key.name.as_str(), key.line, key.double_quoted))
            .collect();

        assert_eq!(
            scanned,
            [
                ("id", 2, false),
                ("title", 3, true),
                ("summary", 4, false),
                ("repos", 6, false),
                ("work", 8, false),
            ]
        );
    }
}
