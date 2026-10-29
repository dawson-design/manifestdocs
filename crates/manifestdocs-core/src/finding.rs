//! Problems that `check` reports, and where it found them.

use std::fmt;

use crate::id::NodeId;
use crate::paths::TreePath;
use crate::record::{DecisionId, RECORD_TOKENS, Topic};
use crate::tokens::grouped;

/// One problem in one file. `file` can name a folder, such as a skipped nested tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub file: TreePath,
    pub line: Option<u32>,
    pub problem: Problem,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// A node frontmatter field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Id,
    Title,
    Status,
    Summary,
    Repos,
    Work,
    SupersededBy,
    Node,
    Topic,
    DecidedBy,
    Date,
    Source,
    AppliesTo,
    Supersedes,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    UnparsableFrontmatter(String),
    MissingField(Field),
    EmptyField(Field),
    UnquotedField(Field),
    WrongType {
        field: Field,
        expected: &'static str,
    },
    MalformedId(String),
    MalformedRepoRef(String),
    MalformedIssueRef(String),
    DuplicateId {
        also_in: TreePath,
    },
    UnknownStatus(String),
    SupersededWithoutReplacement,
    ReplacementWithoutSuperseded,
    SupersededBySelf,
    ReplacementNotFound(NodeId),
    NodeIdOutsideNodeFile(NodeId),
    BrokenLink(String),
    /// `path` is the link's path as written, and `anchor` the part after `#`.
    MissingAnchor {
        path: String,
        anchor: String,
    },
    NoRootNode {
        expected: TreePath,
    },
    NestedTreeSkipped,
    UnknownField(String),
    NodeFileWithoutId,
    /// A node with child nodes has a body estimated at `tokens`, over the tree's `node_tokens`.
    NodeTooLarge {
        tokens: u32,
        limit: u32,
    },
    MalformedDecisionId(String),
    DuplicateDecisionId {
        also_in: TreePath,
    },
    UnknownRecordStatus(String),
    RecordNodeNotFound(NodeId),
    /// The record's `node` isn't the node owning the `decisions/` folder it's in.
    RecordInWrongFolder {
        node: NodeId,
    },
    SupersededRecordNotFound(DecisionId),
    AcceptedTopicTwice {
        topic: Topic,
        also_in: TreePath,
    },
    /// A record's body is estimated at `tokens`, over the fixed `RECORD_TOKENS`.
    RecordTooLarge {
        tokens: u32,
    },
}

impl Problem {
    pub fn severity(&self) -> Severity {
        match self {
            Self::NoRootNode { .. }
            | Self::NestedTreeSkipped
            | Self::UnknownField(_)
            | Self::NodeFileWithoutId
            | Self::NodeTooLarge { .. }
            | Self::RecordTooLarge { .. } => Severity::Warning,
            Self::UnparsableFrontmatter(_)
            | Self::MissingField(_)
            | Self::EmptyField(_)
            | Self::UnquotedField(_)
            | Self::WrongType { .. }
            | Self::MalformedId(_)
            | Self::MalformedRepoRef(_)
            | Self::MalformedIssueRef(_)
            | Self::DuplicateId { .. }
            | Self::UnknownStatus(_)
            | Self::SupersededWithoutReplacement
            | Self::ReplacementWithoutSuperseded
            | Self::SupersededBySelf
            | Self::ReplacementNotFound(_)
            | Self::NodeIdOutsideNodeFile(_)
            | Self::BrokenLink(_)
            | Self::MissingAnchor { .. }
            | Self::MalformedDecisionId(_)
            | Self::DuplicateDecisionId { .. }
            | Self::UnknownRecordStatus(_)
            | Self::RecordNodeNotFound(_)
            | Self::RecordInWrongFolder { .. }
            | Self::SupersededRecordNotFound(_)
            | Self::AcceptedTopicTwice { .. } => Severity::Error,
        }
    }
}

impl Field {
    pub fn name(self) -> &'static str {
        match self {
            Self::Id => "id",
            Self::Title => "title",
            Self::Status => "status",
            Self::Summary => "summary",
            Self::Repos => "repos",
            Self::Work => "work",
            Self::SupersededBy => "superseded_by",
            Self::Node => "node",
            Self::Topic => "topic",
            Self::DecidedBy => "decided_by",
            Self::Date => "date",
            Self::Source => "source",
            Self::AppliesTo => "applies_to",
            Self::Supersedes => "supersedes",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|field| field.name() == name)
    }

    /// The YAML type the field takes, as a problem message names it.
    pub fn expected(self) -> &'static str {
        match self {
            Self::Repos | Self::Work => "a list",
            Self::Id | Self::Title | Self::Status | Self::Summary | Self::SupersededBy => "text",
            Self::Source | Self::AppliesTo => "a list",
            Self::Node | Self::DecidedBy | Self::Supersedes => "text",
            Self::Topic => "kebab-case text",
            Self::Date => "a date, YYYY-MM-DD",
        }
    }

    // Node fields only, so `from_name` still treats a node's `topic` as an unknown field.
    const ALL: [Field; 7] = [
        Self::Id,
        Self::Title,
        Self::Status,
        Self::Summary,
        Self::Repos,
        Self::Work,
        Self::SupersededBy,
    ];
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Error => "error",
            Self::Warning => "warning",
        })
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnparsableFrontmatter(detail) => {
                write!(f, "frontmatter isn't valid YAML: {detail}")
            }
            Self::MissingField(field) => write!(f, "missing `{}`", field.name()),
            Self::EmptyField(field) => write!(f, "`{}` is empty", field.name()),
            Self::UnquotedField(field) => write!(f, "`{}` must be double-quoted", field.name()),
            Self::WrongType { field, expected } => {
                write!(f, "`{}` must be {expected}", field.name())
            }
            Self::MalformedId(text) => {
                write!(
                    f,
                    "malformed node ID `{text}`: expected `fd-` and 6 lowercase hex characters"
                )
            }
            Self::MalformedRepoRef(text) => {
                write!(f, "malformed repo `{text}`: expected `owner/name`")
            }
            Self::MalformedIssueRef(text) => write!(
                f,
                "malformed work item `{text}`: expected `owner/name#number`"
            ),
            Self::DuplicateId { also_in } => write!(f, "ID is also used by `{also_in}`"),
            Self::UnknownStatus(status) => write!(
                f,
                "status `{status}` isn't in `statuses` in manifestdocs.yaml"
            ),
            Self::SupersededWithoutReplacement => {
                f.write_str("status is `superseded` but `superseded_by` is missing")
            }
            Self::ReplacementWithoutSuperseded => {
                f.write_str("`superseded_by` is set but status isn't `superseded`")
            }
            Self::SupersededBySelf => f.write_str("`superseded_by` names this node"),
            Self::ReplacementNotFound(id) => write!(
                f,
                "`superseded_by` names `{id}`, which isn't a node in this tree"
            ),
            Self::NodeIdOutsideNodeFile(id) => {
                write!(
                    f,
                    "node ID `{id}` in a file that isn't named after its folder"
                )
            }
            Self::BrokenLink(target) => write!(f, "broken link `{target}`"),
            Self::MissingAnchor { path, anchor } => write!(f, "no heading `#{anchor}` in `{path}`"),
            Self::NoRootNode { expected } => write!(f, "no root node `{expected}`"),
            Self::NestedTreeSkipped => f.write_str("skipped nested tree"),
            Self::UnknownField(name) => write!(f, "unknown field `{name}`"),
            Self::NodeFileWithoutId => f.write_str("node file has frontmatter but no `id`"),
            Self::NodeTooLarge { tokens, limit } => write!(
                f,
                "node body is about {} tokens, over `node_tokens` ({}); move detail into child nodes or a linked supporting doc",
                grouped(*tokens),
                grouped(*limit)
            ),
            Self::MalformedDecisionId(text) => write!(
                f,
                "malformed decision ID `{text}`: expected `dc-` and 7 lowercase hex characters"
            ),
            Self::DuplicateDecisionId { also_in } => {
                write!(f, "decision ID is also used by `{also_in}`")
            }
            Self::UnknownRecordStatus(status) => write!(
                f,
                "status `{status}` isn't a record status: expected `proposed`, `accepted`, `superseded` or `rejected`"
            ),
            Self::RecordNodeNotFound(id) => {
                write!(f, "`node` names `{id}`, which isn't a node in this tree")
            }
            Self::RecordInWrongFolder { node } => write!(
                f,
                "`node` is `{node}`, but the record's `decisions/` folder isn't that node's"
            ),
            Self::SupersededRecordNotFound(id) => write!(
                f,
                "`supersedes` names `{id}`, which isn't a decision record in this tree"
            ),
            Self::AcceptedTopicTwice { topic, also_in } => write!(
                f,
                "another accepted record, `{also_in}`, has topic `{topic}`"
            ),
            Self::RecordTooLarge { tokens } => write!(
                f,
                "record body is about {} tokens, over {}; a choice that needs more is a spec change or an ADR",
                grouped(*tokens),
                grouped(RECORD_TOKENS)
            ),
        }
    }
}
