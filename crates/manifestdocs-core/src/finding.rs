//! Problems that `check` reports, and where it found them.

use std::fmt;

use crate::id::NodeId;
use crate::paths::TreePath;

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
    MissingAnchor(String),
    NoRootNode {
        expected: TreePath,
    },
    NestedTreeSkipped,
    UnknownField(String),
    NodeFileWithoutId,
}

impl Problem {
    pub fn severity(&self) -> Severity {
        match self {
            Self::NoRootNode { .. }
            | Self::NestedTreeSkipped
            | Self::UnknownField(_)
            | Self::NodeFileWithoutId => Severity::Warning,
            _ => Severity::Error,
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
        }
    }
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
            Self::MissingAnchor(target) => write!(f, "no heading matches the anchor in `{target}`"),
            Self::NoRootNode { expected } => write!(f, "no root node `{expected}`"),
            Self::NestedTreeSkipped => f.write_str("skipped nested tree"),
            Self::UnknownField(name) => write!(f, "unknown field `{name}`"),
            Self::NodeFileWithoutId => f.write_str("node file has frontmatter but no `id`"),
        }
    }
}
