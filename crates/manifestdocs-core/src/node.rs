//! Feature nodes: a folder's node file, parsed from its frontmatter.

use std::collections::BTreeMap;

use crate::config::{SUPERSEDED, StatusName, TreeConfig};
use crate::finding::{Field, Finding, Problem};
use crate::frontmatter::{self, FIRST_YAML_LINE, Frontmatter, Key, Split, Value};
use crate::id::NodeId;
use crate::paths::TreePath;
use crate::refs::{IssueRef, RepoRef};
use crate::yaml;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    pub location: NodeLocation,
    pub title: NonEmptyString,
    pub status: NodeStatus,
    pub summary: NonEmptyString,
    pub repos: Vec<RepoRef>,
    pub work: Vec<IssueRef>,
    pub body: String,
}

/// Where a node is, taken from its file's place in the tree, never from frontmatter.
/// `Folder` always names a folder below the root; the root itself is `Root`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeLocation {
    Root,
    Folder(TreePath),
}

impl NodeLocation {
    /// The node's folder, which is empty for the root node.
    pub fn folder(&self) -> TreePath {
        match self {
            NodeLocation::Root => TreePath::default(),
            NodeLocation::Folder(folder) => folder.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeStatus {
    Active(StatusName),
    Superseded { by: NodeId },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonEmptyString(String);

/// A file in a node file's position, read whether or not its frontmatter is valid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedNode {
    pub file: TreePath,
    pub location: NodeLocation,
    /// Set whenever the `id` parses, even if another field doesn't, so references to it resolve.
    pub id: Option<NodeId>,
    pub node: Option<Node>,
    pub findings: Vec<Finding>,
    /// The frontmatter's top-level keys, so tree-wide problems can point at a field's line.
    pub keys: Vec<Key>,
}

impl ParsedNode {
    /// The line of `field`'s key, or the opening `---` when the field is absent.
    pub fn line_of(&self, field: Field) -> u32 {
        let key = self.keys.iter().find(|key| key.name == field.name());
        key.map_or(OPENING_LINE, |key| key.line)
    }
}

/// Problems that belong to the whole block, such as a missing field, point at the opening `---`.
pub(crate) const OPENING_LINE: u32 = 1;

impl NonEmptyString {
    pub fn new(text: String) -> Option<Self> {
        (!text.trim().is_empty()).then_some(Self(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Reads a file in a node file's position. `None` when it has no frontmatter, which makes it
/// a supporting doc.
pub fn read_node_file(
    file: &TreePath,
    location: NodeLocation,
    text: &str,
    config: &TreeConfig,
) -> Option<ParsedNode> {
    match frontmatter::split(text) {
        Split::Absent => None,
        // The scanned `id` keeps the node in the tree, so one deleted line doesn't hide it.
        Split::Unclosed { keys } => {
            let problem = Problem::UnparsableFrontmatter("no closing `---` line".into());
            let rejected = rejected(file, location, problem);
            Some(ParsedNode {
                id: scanned_id(&keys),
                keys,
                ..rejected
            })
        }
        Split::Present(frontmatter) => Some(parse_node(file, location, &frontmatter, config)),
    }
}

/// The node ID a Markdown file's frontmatter declares, whether or not it's a node file.
pub fn declared_id(frontmatter: &Frontmatter) -> Option<NodeId> {
    match frontmatter::values(frontmatter.yaml) {
        Ok(values) => match values.get(Field::Id.name()) {
            Some(Value::Text(text)) => NodeId::parse(text),
            _ => None,
        },
        Err(_) => scanned_id(&frontmatter.keys),
    }
}

fn rejected(file: &TreePath, location: NodeLocation, problem: Problem) -> ParsedNode {
    let finding = Finding {
        file: file.clone(),
        line: Some(OPENING_LINE),
        problem,
    };
    ParsedNode {
        file: file.clone(),
        location,
        id: None,
        node: None,
        findings: vec![finding],
        keys: Vec::new(),
    }
}

fn parse_node(
    file: &TreePath,
    location: NodeLocation,
    frontmatter: &Frontmatter,
    config: &TreeConfig,
) -> ParsedNode {
    let mut reader = Reader {
        file,
        frontmatter,
        findings: Vec::new(),
    };
    let (id, node) = match frontmatter::values(frontmatter.yaml) {
        Ok(values) if !values.contains_key(Field::Id.name()) => {
            return rejected(file, location, Problem::NodeFileWithoutId);
        }
        Ok(values) => reader.fields(&values, location.clone(), config),
        Err(error) => {
            reader.unparsable(&error);
            (scanned_id(&frontmatter.keys), None)
        }
    };
    ParsedNode {
        file: file.clone(),
        location,
        id,
        node,
        findings: reader.findings,
        keys: frontmatter.keys.clone(),
    }
}

// When the YAML doesn't parse, the scanned `id` still lets references to the node resolve.
fn scanned_id(keys: &[Key]) -> Option<NodeId> {
    NodeId::parse(scanned_text(keys, Field::Id)?)
}

/// A field's value as scanned from the text, without quotes or a trailing comment.
pub(crate) fn scanned_text(keys: &[Key], field: Field) -> Option<&str> {
    let key = keys.iter().find(|key| key.name == field.name())?;
    let without_comment = key.value.split(" #").next().unwrap_or_default();
    Some(without_comment.trim().trim_matches(['"', '\'']))
}

pub(crate) fn value(values: &BTreeMap<String, Value>, field: Field) -> Option<&Value> {
    values.get(field.name())
}

/// Collects every problem in one file's frontmatter. Decision records use it too.
pub(crate) struct Reader<'a> {
    pub(crate) file: &'a TreePath,
    pub(crate) frontmatter: &'a Frontmatter<'a>,
    pub(crate) findings: Vec<Finding>,
}

impl Reader<'_> {
    pub(crate) fn report(&mut self, line: u32, problem: Problem) {
        self.findings.push(Finding {
            file: self.file.clone(),
            line: Some(line),
            problem,
        });
    }

    pub(crate) fn report_field(&mut self, field: Field, problem: Problem) {
        let key = self.frontmatter.key(field.name());
        self.report(key.map_or(OPENING_LINE, |key| key.line), problem);
    }

    // The quoting check still runs, since a scan of the text doesn't need the YAML to parse.
    fn unparsable(&mut self, error: &serde_saphyr::Error) {
        let problem = Problem::UnparsableFrontmatter(yaml::error_message(error));
        self.report(file_line(error), problem);
        self.check_quoting(Field::Title);
        self.check_quoting(Field::Summary);
    }

    fn fields(
        &mut self,
        values: &BTreeMap<String, Value>,
        location: NodeLocation,
        config: &TreeConfig,
    ) -> (Option<NodeId>, Option<Node>) {
        for name in values
            .keys()
            .filter(|name| Field::from_name(name).is_none())
        {
            let line = self
                .frontmatter
                .key(name)
                .map_or(OPENING_LINE, |key| key.line);
            self.report(line, Problem::UnknownField(name.clone()));
        }
        let id = self.id(value(values, Field::Id));
        let node = self.node(values, id.clone(), location, config);
        (id, node)
    }

    fn node(
        &mut self,
        values: &BTreeMap<String, Value>,
        id: Option<NodeId>,
        location: NodeLocation,
        config: &TreeConfig,
    ) -> Option<Node> {
        let title = self.quoted_text(Field::Title, values);
        let summary = self.quoted_text(Field::Summary, values);
        let status = self.status(values, id.as_ref(), config);
        let repos = value(values, Field::Repos);
        let repos = self.list(
            Field::Repos,
            repos,
            RepoRef::parse,
            Problem::MalformedRepoRef,
        );
        let work = value(values, Field::Work);
        let work = self.list(
            Field::Work,
            work,
            IssueRef::parse,
            Problem::MalformedIssueRef,
        );
        Some(Node {
            id: id?,
            location,
            title: title?,
            status: status?,
            summary: summary?,
            repos,
            work,
            body: self.frontmatter.body.to_owned(),
        })
    }

    fn id(&mut self, value: Option<&Value>) -> Option<NodeId> {
        let text = self.text(Field::Id, value)?;
        let id = NodeId::parse(text.as_str());
        if id.is_none() {
            self.report_field(Field::Id, Problem::MalformedId(text.0));
        }
        id
    }

    pub(crate) fn quoted_text(
        &mut self,
        field: Field,
        values: &BTreeMap<String, Value>,
    ) -> Option<NonEmptyString> {
        let value = value(values, field);
        if matches!(value, Some(Value::Text(_))) {
            self.check_quoting(field);
        }
        self.text(field, value)
    }

    // A value that starts on the next line can't be judged from the key's line, so it's left.
    pub(crate) fn check_quoting(&mut self, field: Field) {
        let Some(key) = self.frontmatter.key(field.name()) else {
            return;
        };
        if !key.value.is_empty() && !key.double_quoted {
            self.report(key.line, Problem::UnquotedField(field));
        }
    }

    pub(crate) fn text(&mut self, field: Field, value: Option<&Value>) -> Option<NonEmptyString> {
        let problem = match value {
            Some(Value::Text(text)) => match NonEmptyString::new(text.clone()) {
                Some(text) => return Some(text),
                None => Problem::EmptyField(field),
            },
            None => Problem::MissingField(field),
            Some(Value::Null) => Problem::EmptyField(field),
            Some(Value::List(_) | Value::Other) => Problem::WrongType {
                field,
                expected: field.expected(),
            },
        };
        self.report_field(field, problem);
        None
    }

    /// `Err` when the value has the wrong type, which is reported.
    pub(crate) fn optional_text(
        &mut self,
        field: Field,
        value: Option<&Value>,
    ) -> Result<Option<String>, ()> {
        match value {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Text(text)) => Ok(Some(text.clone())),
            Some(Value::List(_) | Value::Other) => {
                let expected = field.expected();
                self.report_field(field, Problem::WrongType { field, expected });
                Err(())
            }
        }
    }

    pub(crate) fn list<T>(
        &mut self,
        field: Field,
        value: Option<&Value>,
        parse: impl Fn(&str) -> Option<T>,
        malformed: impl Fn(String) -> Problem,
    ) -> Vec<T> {
        let items = match value {
            None | Some(Value::Null) => return Vec::new(),
            Some(Value::List(items)) => items,
            Some(Value::Text(_) | Value::Other) => {
                let expected = field.expected();
                self.report_field(field, Problem::WrongType { field, expected });
                return Vec::new();
            }
        };
        let mut parsed = Vec::new();
        for text in items {
            match parse(text) {
                Some(item) => parsed.push(item),
                None => self.report_field(field, malformed(text.clone())),
            }
        }
        parsed
    }

    fn status(
        &mut self,
        values: &BTreeMap<String, Value>,
        id: Option<&NodeId>,
        config: &TreeConfig,
    ) -> Option<NodeStatus> {
        let status = self.text(Field::Status, value(values, Field::Status));
        let superseded_by = value(values, Field::SupersededBy);
        let superseded_by = self.optional_text(Field::SupersededBy, superseded_by);
        let (status, superseded_by) = (status?, superseded_by.ok()?);
        if status.as_str() == SUPERSEDED {
            return self.replacement(superseded_by, id);
        }
        if superseded_by.is_some() {
            self.report_field(Field::SupersededBy, Problem::ReplacementWithoutSuperseded);
        }
        let known = config
            .statuses
            .iter()
            .find(|name| name.as_str() == status.as_str());
        if known.is_none() {
            self.report_field(Field::Status, Problem::UnknownStatus(status.0.clone()));
        }
        known.cloned().map(NodeStatus::Active)
    }

    fn replacement(
        &mut self,
        superseded_by: Option<String>,
        id: Option<&NodeId>,
    ) -> Option<NodeStatus> {
        let Some(text) = superseded_by else {
            self.report_field(Field::Status, Problem::SupersededWithoutReplacement);
            return None;
        };
        let Some(by) = NodeId::parse(&text) else {
            self.report_field(Field::SupersededBy, Problem::MalformedId(text));
            return None;
        };
        if Some(&by) == id {
            self.report_field(Field::SupersededBy, Problem::SupersededBySelf);
            return None;
        }
        Some(NodeStatus::Superseded { by })
    }
}

/// The file line of a YAML error inside the frontmatter.
pub(crate) fn file_line(error: &serde_saphyr::Error) -> u32 {
    yaml::error_line(error).map_or(OPENING_LINE, |line| line + FIRST_YAML_LINE - 1)
}
