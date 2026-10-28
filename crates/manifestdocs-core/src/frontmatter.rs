//! The YAML block between `---` lines at the top of a Markdown file.

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{Deserialize, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};

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
    /// The file line the body starts on.
    pub body_line: u32,
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
            let body_start = offset + line.len();
            return Split::Present(Frontmatter {
                yaml,
                body: &text[body_start..],
                body_line: line_number(text, body_start),
                keys: keys(yaml),
            });
        }
        offset += line.len();
    }
    Split::Unclosed
}

/// The Markdown after any frontmatter, and the file line it starts on. A block that never
/// closes counts as body, so its links are still checked.
pub fn body(text: &str) -> (&str, u32) {
    match split(text) {
        Split::Present(frontmatter) => (frontmatter.body, frontmatter.body_line),
        Split::Absent | Split::Unclosed => (text.strip_prefix('\u{feff}').unwrap_or(text), 1),
    }
}

impl Frontmatter<'_> {
    pub fn key(&self, name: &str) -> Option<&Key> {
        self.keys.iter().find(|key| key.name == name)
    }
}

fn line_number(text: &str, offset: usize) -> u32 {
    let newlines = text[..offset].matches('\n').count();
    u32::try_from(newlines + 1).unwrap_or(u32::MAX)
}

fn is_fence(line: &str) -> bool {
    line.trim_end() == "---"
}

fn keys(yaml: &str) -> Vec<Key> {
    let lines = (FIRST_YAML_LINE..).zip(yaml.lines());
    lines.filter_map(|(line, text)| key(line, text)).collect()
}

// A top-level key starts the line, so indented, list and comment lines are skipped. YAML
// also allows a quoted key and spaces before the colon.
fn key(line: u32, text: &str) -> Option<Key> {
    let (name, value) = text.split_once(':')?;
    let name = unquote(name.trim_end());
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

fn unquote(name: &str) -> &str {
    let quoted = ['"', '\''].into_iter().find_map(|quote| {
        let inner = name.strip_prefix(quote)?;
        inner.strip_suffix(quote)
    });
    quoted.unwrap_or(name)
}

/// One frontmatter value, read without assuming its field's type, so each field's type can
/// be checked on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Null,
    Text(String),
    /// A list of scalars, each as text.
    List(Vec<String>),
    /// A mapping, or a list holding anything but scalars.
    Other,
}

/// The YAML's top-level fields, or the parser's error. An empty block has no fields.
pub fn values(yaml: &str) -> Result<BTreeMap<String, Value>, serde_saphyr::Error> {
    let values: Option<BTreeMap<String, Value>> = serde_saphyr::from_str(yaml)?;
    Ok(values.unwrap_or_default())
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ValueVisitor)
    }
}

// Numbers and booleans are read back as text, since every node field is text or a list of it.
struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a frontmatter value")
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_str<E>(self, text: &str) -> Result<Value, E> {
        Ok(Value::Text(text.to_owned()))
    }

    fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Text(value.to_string()))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Text(value.to_string()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Text(value.to_string()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
        Ok(Value::Text(value.to_string()))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut items = Vec::new();
        let mut scalars_only = true;
        while let Some(item) = seq.next_element::<Value>()? {
            match item {
                Value::Text(text) => items.push(text),
                _ => scalars_only = false,
            }
        }
        Ok(if scalars_only {
            Value::List(items)
        } else {
            Value::Other
        })
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(Value::Other)
    }
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
        assert_eq!(frontmatter.body_line, 4);
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

    #[test]
    fn scans_quoted_keys_and_keys_with_a_space_before_the_colon() {
        let names: Vec<String> = present("---\n\"id\": x\ntitle : y\n'work': []\n---\n")
            .keys
            .into_iter()
            .map(|key| key.name)
            .collect();

        assert_eq!(names, ["id", "title", "work"]);
    }

    #[test]
    fn reads_each_value_without_assuming_its_type() {
        let yaml = "id: fd-000001\ncount: 123456\nflag: true\nempty:\nrepos: [a/b, 7]\nnested: {a: b}\nmixed: [a, [b]]\n";
        let values = values(yaml).unwrap();
        let text = |s: &str| Value::Text(s.to_owned());

        assert_eq!(values["id"], text("fd-000001"));
        assert_eq!(values["count"], text("123456"));
        assert_eq!(values["flag"], text("true"));
        assert_eq!(values["empty"], Value::Null);
        assert_eq!(values["repos"], Value::List(vec!["a/b".into(), "7".into()]));
        assert_eq!(values["nested"], Value::Other);
        assert_eq!(values["mixed"], Value::Other);
    }

    #[test]
    fn reads_an_empty_or_comment_only_block_as_no_fields() {
        assert_eq!(values("").unwrap(), BTreeMap::new());
        assert_eq!(values("# just a note\n").unwrap(), BTreeMap::new());
        assert!(values("id: [a\n").is_err());
        assert!(values("just text\n").is_err());
    }
}
