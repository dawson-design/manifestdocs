//! Feature nodes: a folder's node file, parsed from its frontmatter.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::de::IgnoredAny;

use crate::config::{SUPERSEDED, StatusName, TreeConfig};
use crate::finding::{Field, Finding, Problem};
use crate::frontmatter::{self, FIRST_YAML_LINE, Frontmatter, Split};
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
    pub location: NodeLocation,
    /// Set whenever the `id` parses, even if another field doesn't, so references to it resolve.
    pub id: Option<NodeId>,
    pub node: Option<Node>,
    pub findings: Vec<Finding>,
}

#[derive(Deserialize)]
struct RawNode {
    id: Option<String>,
    title: Option<String>,
    status: Option<String>,
    summary: Option<String>,
    repos: Option<Vec<String>>,
    work: Option<Vec<String>>,
    superseded_by: Option<String>,
}

/// Problems that belong to the whole block, such as a missing field, point at the opening `---`.
const OPENING_LINE: u32 = 1;

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
    let frontmatter = match frontmatter::split(text) {
        Split::Absent => return None,
        Split::Unclosed => {
            let problem = Problem::UnparsableFrontmatter("no closing `---` line".into());
            return Some(rejected(file, location, problem));
        }
        Split::Present(frontmatter) => frontmatter,
    };
    if frontmatter.key(Field::Id.name()).is_none() {
        return Some(rejected(file, location, Problem::NodeFileWithoutId));
    }
    Some(parse_node(file, location, &frontmatter, config))
}

fn rejected(file: &TreePath, location: NodeLocation, problem: Problem) -> ParsedNode {
    let finding = Finding {
        file: file.clone(),
        line: Some(OPENING_LINE),
        problem,
    };
    ParsedNode {
        location,
        id: None,
        node: None,
        findings: vec![finding],
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
    reader.scan_keys();
    let (id, node) = match reader.read_raw() {
        Some(raw) => {
            let id = reader.id(raw.id.clone());
            let node = reader.node(raw, id.clone(), location.clone(), config);
            (id, node)
        }
        None => (scanned_id(frontmatter), None),
    };
    ParsedNode {
        location,
        id,
        node,
        findings: reader.findings,
    }
}

// When the YAML doesn't parse, the scanned `id` still lets references to the node resolve.
fn scanned_id(frontmatter: &Frontmatter) -> Option<NodeId> {
    let key = frontmatter.key(Field::Id.name())?;
    NodeId::parse(key.value.trim_matches(['"', '\'']))
}

/// Collects every problem in one node file's frontmatter.
struct Reader<'a> {
    file: &'a TreePath,
    frontmatter: &'a Frontmatter<'a>,
    findings: Vec<Finding>,
}

impl Reader<'_> {
    fn report(&mut self, line: u32, problem: Problem) {
        self.findings.push(Finding {
            file: self.file.clone(),
            line: Some(line),
            problem,
        });
    }

    fn report_field(&mut self, field: Field, problem: Problem) {
        let key = self.frontmatter.key(field.name());
        self.report(key.map_or(OPENING_LINE, |key| key.line), problem);
    }

    fn scan_keys(&mut self) {
        let frontmatter = self.frontmatter;
        for key in &frontmatter.keys {
            match Field::from_name(&key.name) {
                Some(field @ (Field::Title | Field::Summary)) if !key.double_quoted => {
                    self.report(key.line, Problem::UnquotedField(field));
                }
                Some(_) => {}
                None => self.report(key.line, Problem::UnknownField(key.name.clone())),
            }
        }
    }

    // A mapping that parses but doesn't fit `RawNode` has a field of the wrong type.
    fn read_raw(&mut self) -> Option<RawNode> {
        let yaml = self.frontmatter.yaml;
        if let Err(error) = serde_saphyr::from_str::<BTreeMap<String, IgnoredAny>>(yaml) {
            let problem = Problem::UnparsableFrontmatter(yaml::error_message(&error));
            self.report(file_line(&error), problem);
            return None;
        }
        serde_saphyr::from_str::<RawNode>(yaml)
            .map_err(|error| self.report_wrong_type(&error))
            .ok()
    }

    fn report_wrong_type(&mut self, error: &serde_saphyr::Error) {
        let line = file_line(error);
        let owner = self.frontmatter.keys.iter().rfind(|key| key.line <= line);
        match owner.and_then(|key| Some((Field::from_name(&key.name)?, key.line))) {
            Some((field, key_line)) => {
                let expected = field.expected();
                self.report(key_line, Problem::WrongType { field, expected });
            }
            None => self.report(
                line,
                Problem::UnparsableFrontmatter(yaml::error_message(error)),
            ),
        }
    }

    fn node(
        &mut self,
        raw: RawNode,
        id: Option<NodeId>,
        location: NodeLocation,
        config: &TreeConfig,
    ) -> Option<Node> {
        let title = self.text(Field::Title, raw.title);
        let summary = self.text(Field::Summary, raw.summary);
        let status = self.status(raw.status, raw.superseded_by, id.as_ref(), config);
        let repos = self.list(
            Field::Repos,
            raw.repos,
            RepoRef::parse,
            Problem::MalformedRepoRef,
        );
        let work = self.list(
            Field::Work,
            raw.work,
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

    fn id(&mut self, raw: Option<String>) -> Option<NodeId> {
        let Some(text) = raw else {
            self.report_field(Field::Id, Problem::MissingField(Field::Id));
            return None;
        };
        let id = NodeId::parse(&text);
        if id.is_none() {
            self.report_field(Field::Id, Problem::MalformedId(text));
        }
        id
    }

    fn text(&mut self, field: Field, raw: Option<String>) -> Option<NonEmptyString> {
        let Some(text) = raw else {
            self.report_field(field, Problem::MissingField(field));
            return None;
        };
        let text = NonEmptyString::new(text);
        if text.is_none() {
            self.report_field(field, Problem::EmptyField(field));
        }
        text
    }

    fn list<T>(
        &mut self,
        field: Field,
        raw: Option<Vec<String>>,
        parse: impl Fn(&str) -> Option<T>,
        malformed: impl Fn(String) -> Problem,
    ) -> Vec<T> {
        let mut values = Vec::new();
        for text in raw.unwrap_or_default() {
            match parse(&text) {
                Some(value) => values.push(value),
                None => self.report_field(field, malformed(text)),
            }
        }
        values
    }

    fn status(
        &mut self,
        raw: Option<String>,
        superseded_by: Option<String>,
        id: Option<&NodeId>,
        config: &TreeConfig,
    ) -> Option<NodeStatus> {
        let status = self.text(Field::Status, raw)?;
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
fn file_line(error: &serde_saphyr::Error) -> u32 {
    yaml::error_line(error).map_or(OPENING_LINE, |line| line + FIRST_YAML_LINE - 1)
}
