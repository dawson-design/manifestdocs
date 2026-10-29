//! `decide`: checks one record an author wrote, fills in its ID, date and status, and says
//! where it goes, or lists every problem that stops it being written.

use std::collections::BTreeSet;
use std::fmt;

use crate::config::MergeMode;
use crate::finding::{Field, Finding, Problem};
use crate::frontmatter::{self, Frontmatter, Key, Split};
use crate::id::NodeId;
use crate::layout::RECORDS_FOLDER;
use crate::node::{OPENING_LINE, scanned_text};
use crate::paths::TreePath;
use crate::record::{
    Date, DecisionId, ParsedRecord, Record, RecordStatus, Topic, read_record_file,
};
use crate::tree::Tree;

/// How `decide` runs, which sets the status it writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Setting {
    pub merge: MergeMode,
    pub unattended: bool,
}

/// What `decide` fills in when the record doesn't give it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filled {
    pub id: DecisionId,
    pub date: Date,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome<'a> {
    New(Box<Draft>),
    /// A record with the same node, topic and body is already in the tree or the folder.
    Unchanged(&'a ParsedRecord),
    /// Sorted by line.
    Invalid(Vec<InputError>),
}

/// A record ready to write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draft {
    pub record: Record,
    pub text: String,
    /// The node's `decisions/` folder, from the tree root.
    pub folder: TreePath,
    /// `YYYY-MM-DD-<topic>`, the file name without `.md`.
    pub file_stem: String,
    /// Problems that don't stop the write, such as a long body.
    pub warnings: Vec<Problem>,
}

/// One problem in the input, at its line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputError {
    pub line: u32,
    /// The frontmatter field the problem is in, when it's in one.
    pub field: Option<String>,
    pub problem: InputProblem,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputProblem {
    /// A problem the record parser or `check` reports.
    Record(Problem),
    NoFrontmatter,
    /// The frontmatter has no closing `---` line.
    Unclosed,
    SuppliedId,
    StatusNotAllowed {
        status: RecordStatus,
        setting: Setting,
    },
    /// The node already has a different record on the topic, which this one doesn't supersede.
    DifferentDecision {
        topic: Topic,
        existing: DecisionId,
    },
    /// A text field holds characters the record can't be written back with.
    Unwritable,
}

impl Setting {
    /// The status `decide` writes when the record doesn't give one.
    pub fn status(self) -> RecordStatus {
        match (self.merge, self.unattended) {
            (MergeMode::Review, false) => RecordStatus::Accepted,
            _ => RecordStatus::Proposed,
        }
    }

    // `proposed` always, since a later PR can accept it, and otherwise only what decide writes.
    fn allows(self, status: RecordStatus) -> bool {
        status == RecordStatus::Proposed || status == self.status()
    }

    fn reason(self) -> &'static str {
        match (self.unattended, self.merge) {
            (true, _) => "in an unattended run",
            (false, MergeMode::Review) => "under `decisions.merge: review`",
            (false, MergeMode::Proposed) => "under `decisions.merge: proposed`",
        }
    }
}

/// Checks `input`, a record in the on-disk format, against the tree and `records`: the tree's
/// records and any already in the folder `decide` writes to.
pub fn decide<'a>(
    input: &str,
    setting: Setting,
    filled: &Filled,
    tree: &Tree,
    records: &'a [ParsedRecord],
) -> Outcome<'a> {
    let input = lf(input);
    let frontmatter = match frontmatter::split(&input) {
        Split::Present(frontmatter) => frontmatter,
        Split::Absent => return invalid(OPENING_LINE, None, InputProblem::NoFrontmatter),
        Split::Unclosed { .. } => return invalid(OPENING_LINE, None, InputProblem::Unclosed),
    };
    let Some(parsed) = parse(&input, &frontmatter, setting, filled) else {
        return invalid(OPENING_LINE, None, InputProblem::NoFrontmatter);
    };
    let keys = &frontmatter.keys;
    let mut errors = field_errors(&parsed, keys, setting);
    errors.extend(reference_errors(keys, tree, records));
    let record = match parsed.record {
        Some(record) if errors.is_empty() => record,
        _ => return Outcome::Invalid(sorted(errors)),
    };
    if let Some(existing) = same_decision(records, &record) {
        return Outcome::Unchanged(existing);
    }
    if let Some(error) = different_decision(records, &record, keys) {
        return Outcome::Invalid(vec![error]);
    }
    draft(record, tree, &parsed.findings, keys)
}

fn invalid<'a>(line: u32, field: Option<String>, problem: InputProblem) -> Outcome<'a> {
    Outcome::Invalid(vec![InputError {
        line,
        field,
        problem,
    }])
}

fn sorted(mut errors: Vec<InputError>) -> Vec<InputError> {
    errors.sort_by_key(|error| error.line);
    errors
}

/// `text` with LF line endings, which `decide` compares bodies by and writes.
fn lf(text: &str) -> String {
    text.replace("\r\n", "\n")
}

// The filled-in fields are added only when the author's YAML parses, so a YAML error is
// reported at the author's own line rather than at one `decide` added.
fn parse(
    input: &str,
    frontmatter: &Frontmatter,
    setting: Setting,
    filled: &Filled,
) -> Option<ParsedRecord> {
    let file = TreePath::new("input.md").unwrap_or_default();
    let text = match frontmatter::values(frontmatter.yaml) {
        Ok(_) => completed(frontmatter, setting, filled),
        Err(_) => input.to_owned(),
    };
    read_record_file(&file, &text)
}

// The filled-in fields go last, so every line the author wrote keeps its number.
fn completed(frontmatter: &Frontmatter, setting: Setting, filled: &Filled) -> String {
    let mut yaml = frontmatter.yaml.to_owned();
    let given = |field: Field| frontmatter.key(field.name()).is_some();
    if !given(Field::Id) {
        yaml.push_str(&format!("id: {}\n", filled.id));
    }
    if !given(Field::Date) {
        yaml.push_str(&format!("date: {}\n", filled.date.as_str()));
    }
    if !given(Field::Status) {
        yaml.push_str(&format!("status: {}\n", setting.status().as_str()));
    }
    format!("---\n{yaml}---\n{}", frontmatter.body)
}

// Every parser finding is an error here, even an unknown field, except a long body.
fn field_errors(parsed: &ParsedRecord, keys: &[Key], setting: Setting) -> Vec<InputError> {
    let id_line = key_line(keys, Field::Id);
    let findings = parsed.findings.iter().filter(|finding| {
        let long = matches!(finding.problem, Problem::RecordTooLarge { .. });
        !long && finding.line != id_line
    });
    let mut errors: Vec<InputError> = findings
        .map(|finding| {
            let line = finding.line.unwrap_or(OPENING_LINE);
            let field = field_of(&finding.problem, line, keys);
            let problem = InputProblem::Record(finding.problem.clone());
            InputError {
                line,
                field,
                problem,
            }
        })
        .collect();
    if let Some(line) = id_line {
        errors.push(at(line, Field::Id, InputProblem::SuppliedId));
    }
    let status = scanned_text(keys, Field::Status).and_then(RecordStatus::parse);
    if let Some(status) = status.filter(|status| !setting.allows(*status)) {
        let line = key_line(keys, Field::Status).unwrap_or(OPENING_LINE);
        let problem = InputProblem::StatusNotAllowed { status, setting };
        errors.push(at(line, Field::Status, problem));
    }
    errors
}

// `node` and `supersedes` must name a node and a record that exist.
fn reference_errors(keys: &[Key], tree: &Tree, records: &[ParsedRecord]) -> Vec<InputError> {
    let mut errors = Vec::new();
    let node = scanned_text(keys, Field::Node).and_then(NodeId::parse);
    if let Some(node) = node.filter(|node| node_folder(tree, node).is_none()) {
        let line = key_line(keys, Field::Node).unwrap_or(OPENING_LINE);
        let problem = InputProblem::Record(Problem::RecordNodeNotFound(node));
        errors.push(at(line, Field::Node, problem));
    }
    let ids: BTreeSet<&DecisionId> = records.iter().filter_map(|r| r.id.as_ref()).collect();
    let older = scanned_text(keys, Field::Supersedes).and_then(DecisionId::parse);
    if let Some(older) = older.filter(|older| !ids.contains(older)) {
        let line = key_line(keys, Field::Supersedes).unwrap_or(OPENING_LINE);
        let problem = InputProblem::Record(Problem::SupersededRecordNotFound(older));
        errors.push(at(line, Field::Supersedes, problem));
    }
    errors
}

fn at(line: u32, field: Field, problem: InputProblem) -> InputError {
    InputError {
        line,
        field: Some(field.name().to_owned()),
        problem,
    }
}

fn key_line(keys: &[Key], field: Field) -> Option<u32> {
    let key = keys.iter().find(|key| key.name == field.name());
    key.map(|key| key.line)
}

// The field a problem names, or else the key on or above its line, such as for a list item.
fn field_of(problem: &Problem, line: u32, keys: &[Key]) -> Option<String> {
    let named = match problem {
        Problem::MissingField(field)
        | Problem::EmptyField(field)
        | Problem::UnquotedField(field)
        | Problem::WrongType { field, .. } => Some(field.name().to_owned()),
        Problem::UnknownField(name) => Some(name.clone()),
        _ => None,
    };
    let above = || {
        let key = keys.iter().rev().find(|key| key.line <= line);
        key.map(|key| key.name.clone())
    };
    named.or_else(above)
}

fn same_topic<'a, 'b>(
    records: &'a [ParsedRecord],
    record: &'b Record,
) -> impl Iterator<Item = (&'a ParsedRecord, &'a Record)> + use<'a, 'b> {
    let parsed = records.iter().filter_map(|p| Some((p, p.record.as_ref()?)));
    parsed.filter(|(_, other)| other.node == record.node && other.topic == record.topic)
}

fn same_decision<'a>(records: &'a [ParsedRecord], record: &Record) -> Option<&'a ParsedRecord> {
    let mut same = same_topic(records, record);
    let body = lf(&record.body);
    let found = same.find(|(_, other)| lf(&other.body).trim() == body.trim());
    found.map(|(parsed, _)| parsed)
}

// A record on the topic that's still in force needs superseding, not a second record. One
// that's already superseded doesn't.
fn different_decision(
    records: &[ParsedRecord],
    record: &Record,
    keys: &[Key],
) -> Option<InputError> {
    let on_topic: Vec<&Record> = same_topic(records, record).map(|(_, r)| r).collect();
    let supersedes_one = on_topic
        .iter()
        .any(|other| record.supersedes.as_ref() == Some(&other.id));
    if supersedes_one {
        return None;
    }
    let in_force = on_topic
        .into_iter()
        .filter(|other| other.status != RecordStatus::Superseded);
    let newest = in_force.max_by(|a, b| (&a.date, &a.id).cmp(&(&b.date, &b.id)))?;
    let line = key_line(keys, Field::Topic).unwrap_or(OPENING_LINE);
    let problem = InputProblem::DifferentDecision {
        topic: record.topic.clone(),
        existing: newest.id.clone(),
    };
    Some(at(line, Field::Topic, problem))
}

fn node_folder(tree: &Tree, node: &NodeId) -> Option<TreePath> {
    let mut entries = tree.entries.values();
    let entry = entries.find(|entry| entry.id == *node)?;
    Some(entry.location.folder())
}

// The text is read back through the record parser, so `decide` never writes a record that
// `check` reads differently.
fn draft<'a>(record: Record, tree: &Tree, findings: &[Finding], keys: &[Key]) -> Outcome<'a> {
    let text = render(&record);
    let file = TreePath::new("input.md").unwrap_or_default();
    let read_back = read_record_file(&file, &text).and_then(|parsed| parsed.record);
    if read_back.as_ref() != Some(&record) {
        let line = key_line(keys, Field::Title).unwrap_or(OPENING_LINE);
        return invalid(line, None, InputProblem::Unwritable);
    }
    let node = node_folder(tree, &record.node).unwrap_or_default();
    let folder = records_folder_of(&node);
    let file_stem = format!("{}-{}", record.date.as_str(), record.topic);
    let long = findings.iter().map(|finding| &finding.problem);
    let warnings = long
        .filter(|problem| matches!(problem, Problem::RecordTooLarge { .. }))
        .cloned()
        .collect();
    Outcome::New(Box::new(Draft {
        record,
        text,
        folder,
        file_stem,
        warnings,
    }))
}

impl Draft {
    /// The file name to try `n`th, from 1: `YYYY-MM-DD-topic.md`, then `-2`, `-3` and so on,
    /// for when another record on the topic has the same date.
    pub fn file_name(&self, n: usize) -> String {
        match n {
            0 | 1 => format!("{}.md", self.file_stem),
            n => format!("{}-{n}.md", self.file_stem),
        }
    }
}

/// `text`, a record file, with its `status` line changed to `status: superseded` and every
/// other byte kept. `None` when its frontmatter has no `status` line.
pub fn mark_superseded(text: &str) -> Option<String> {
    let Split::Present(frontmatter) = frontmatter::split(text) else {
        return None;
    };
    let line = key_line(&frontmatter.keys, Field::Status)?;
    let lines = (1..).zip(text.split_inclusive('\n'));
    let marked = lines.map(|(number, text)| {
        if number != line {
            return text.to_owned();
        }
        let ending = &text[text.trim_end_matches(['\r', '\n']).len()..];
        format!("status: superseded{ending}")
    });
    Some(marked.collect())
}

fn records_folder_of(node_folder: &TreePath) -> TreePath {
    let path = match node_folder.as_str() {
        "" => RECORDS_FOLDER.to_owned(),
        folder => format!("{folder}/{RECORDS_FOLDER}"),
    };
    TreePath::new(&path).unwrap_or_default()
}

/// The record in the on-disk format, with its fields in the order the spec shows them.
/// Fields with no value are left out.
pub fn render(record: &Record) -> String {
    let mut yaml = format!(
        "id: {}\nnode: {}\ntitle: {}\nstatus: {}\ntopic: {}\n",
        record.id,
        record.node,
        quoted(record.title.as_str()),
        record.status.as_str(),
        record.topic,
    );
    if let Some(decided_by) = &record.decided_by {
        yaml.push_str(&format!("decided_by: {}\n", quoted(decided_by)));
    }
    yaml.push_str(&format!("date: {}\n", record.date.as_str()));
    let source: Vec<String> = record.source.iter().map(ToString::to_string).collect();
    let applies_to: Vec<String> = record.applies_to.iter().map(|s| quoted(s)).collect();
    for (name, items) in [("source", source), ("applies_to", applies_to)] {
        if !items.is_empty() {
            yaml.push_str(&format!("{name}: [{}]\n", items.join(", ")));
        }
    }
    if let Some(older) = &record.supersedes {
        yaml.push_str(&format!("supersedes: {older}\n"));
    }
    format!("---\n{yaml}---\n{}", record.body)
}

// A YAML double-quoted scalar. Any character this doesn't escape and YAML changes is caught
// by the read-back in `draft`.
fn quoted(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t");
    format!("\"{escaped}\"")
}

/// The docs clone `decide` writes into, as git reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloneState {
    /// `None` when HEAD is detached.
    pub branch: Option<String>,
    /// From `origin/HEAD`, when it's set.
    pub default_branch: Option<String>,
    /// Whether HEAD has commits that `origin/HEAD` doesn't.
    pub ahead_of_default: bool,
    pub uncommitted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CloneWarning {
    AheadOfDefaultBranch { branch: String, default: String },
    UncommittedChanges,
}

/// What's worth a warning before writing into the clone. A fresh branch from the default
/// branch, such as the decide skill's worktree, isn't. A detached HEAD, or a clone with no
/// known default branch, has no branch to compare.
pub fn clone_warnings(state: &CloneState) -> Vec<CloneWarning> {
    let mut warnings = Vec::new();
    if let (Some(branch), Some(default)) = (&state.branch, &state.default_branch)
        && state.ahead_of_default
    {
        warnings.push(CloneWarning::AheadOfDefaultBranch {
            branch: branch.clone(),
            default: default.clone(),
        });
    }
    if state.uncommitted {
        warnings.push(CloneWarning::UncommittedChanges);
    }
    warnings
}

impl CloneWarning {
    pub fn code(&self) -> &'static str {
        match self {
            Self::AheadOfDefaultBranch { .. } => "ahead_of_default_branch",
            Self::UncommittedChanges => "uncommitted_changes",
        }
    }
}

impl fmt::Display for CloneWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AheadOfDefaultBranch { branch, default } => write!(
                f,
                "the docs clone is on `{branch}`, which has commits that aren't on its default branch `{default}`"
            ),
            Self::UncommittedChanges => f.write_str("the docs clone has uncommitted changes"),
        }
    }
}

impl InputError {
    /// The problem's name in snake case, as JSON output names it.
    pub fn code(&self) -> &'static str {
        match &self.problem {
            InputProblem::Record(problem) => problem.code(),
            InputProblem::NoFrontmatter => "no_frontmatter",
            InputProblem::Unclosed => "unclosed_frontmatter",
            InputProblem::SuppliedId => "supplied_id",
            InputProblem::StatusNotAllowed { .. } => "status_not_allowed",
            InputProblem::DifferentDecision { .. } => "different_decision",
            InputProblem::Unwritable => "unwritable",
        }
    }

    /// What the author changes to fix the problem.
    pub fn fix(&self) -> String {
        let field = self.field.as_deref().unwrap_or("the field");
        match &self.problem {
            InputProblem::Record(problem) => record_fix(problem, field),
            InputProblem::NoFrontmatter => {
                "start it with its fields between `---` lines, as a record file does".into()
            }
            InputProblem::Unclosed => "add a `---` line after the last field".into(),
            InputProblem::SuppliedId => "remove `id`".into(),
            InputProblem::StatusNotAllowed { .. } => "remove `status`, and decide sets it".into(),
            InputProblem::DifferentDecision { existing, .. } => {
                format!("add `supersedes: {existing}` to replace it, or keep its body to rerun it")
            }
            InputProblem::Unwritable => {
                "remove control characters from `title`, `decided_by` and `applies_to`".into()
            }
        }
    }
}

fn record_fix(problem: &Problem, field: &str) -> String {
    match problem {
        Problem::UnparsableFrontmatter(_) => "correct the YAML on this line".into(),
        Problem::MissingField(_) => format!("add `{field}`"),
        Problem::EmptyField(_) => format!("give `{field}` a value"),
        Problem::UnquotedField(_) => format!("put the value of `{field}` in double quotes"),
        Problem::WrongType { expected, .. } => format!("make `{field}` {expected}"),
        Problem::MalformedId(_) | Problem::RecordNodeNotFound(_) => {
            "set `node` to a node ID from `mfd tree`".into()
        }
        Problem::MalformedIssueRef(_) => {
            format!("write each item in `{field}` as `owner/name#number`")
        }
        Problem::MalformedDecisionId(_) | Problem::SupersededRecordNotFound(_) => {
            format!("set `{field}` to a decision ID from `mfd decisions`")
        }
        Problem::UnknownRecordStatus(_) => "remove `status`, and decide sets it".into(),
        Problem::UnknownField(name) => format!("remove `{name}`, or correct its name"),
        _ => format!("correct `{field}`"),
    }
}

impl fmt::Display for InputProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Record(problem) => problem.fmt(f),
            Self::NoFrontmatter => f.write_str("the record has no frontmatter"),
            Self::Unclosed => f.write_str("the frontmatter has no closing `---` line"),
            Self::SuppliedId => f.write_str("`id` is given, but decide generates it"),
            Self::StatusNotAllowed { status, setting } => write!(
                f,
                "status `{}` isn't allowed: decide writes `{}` {}",
                status.as_str(),
                setting.status().as_str(),
                setting.reason()
            ),
            Self::DifferentDecision { topic, existing } => write!(
                f,
                "topic `{topic}` already has a different decision, `{existing}`"
            ),
            Self::Unwritable => f.write_str("the record's text can't be written back as YAML"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_only_the_status_line_superseded_and_keeps_its_line_ending() {
        let text = "---\r\nid: dc-0000001\r\nstatus: accepted # set by review\r\ntopic: t\r\n---\r\n\r\nstatus: accepted\r\n";

        let marked = mark_superseded(text);

        assert_eq!(
            marked.as_deref(),
            Some(
                "---\r\nid: dc-0000001\r\nstatus: superseded\r\ntopic: t\r\n---\r\n\r\nstatus: accepted\r\n"
            )
        );
        assert_eq!(mark_superseded("---\nid: dc-0000001\n---\n"), None);
        assert_eq!(mark_superseded("status: accepted\n"), None);
    }
}
