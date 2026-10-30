//! Decision records: the Markdown in a node's `decisions/` folder, and the rules across them.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::check::{Snapshot, markdown};
use crate::config::TreeConfig;
use crate::finding::{Field, Finding, Problem};
use crate::frontmatter::{self, Frontmatter, Key, Split, Value};
use crate::id::NodeId;
use crate::layout::{layout, records_folder};
use crate::node::{
    NonEmptyString, OPENING_LINE, ParsedNode, Reader, file_line, scanned_text, value,
};
use crate::paths::{RepoPath, TreePath};
use crate::refs::IssueRef;
use crate::tokens::estimate;
use crate::yaml;

/// `check` warns about a record whose body is estimated at more tokens than this, in any tree.
pub const RECORD_TOKENS: u32 = 300;

/// A decision record ID: `dc-` and exactly 7 lowercase hex characters.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecisionId(String);

/// A decision ID or the start of one, as typed on the command line: `dc-` and 4 to 7
/// lowercase hex characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionIdPrefix(String);

/// A record's topic, in kebab case. A node has at most one accepted record per topic.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Topic(String);

/// A date written `YYYY-MM-DD`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date(String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordStatus {
    Proposed,
    Accepted,
    Superseded,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub id: DecisionId,
    /// Must be the node whose `decisions/` folder holds the file.
    pub node: NodeId,
    pub title: NonEmptyString,
    pub status: RecordStatus,
    pub topic: Topic,
    pub decided_by: Option<String>,
    pub date: Date,
    /// Issues and pull requests.
    pub source: Vec<IssueRef>,
    /// Free text for readers, never checked.
    pub applies_to: Vec<String>,
    pub supersedes: Option<DecisionId>,
    pub body: String,
}

/// A record file, read whether or not its frontmatter is valid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedRecord {
    pub file: TreePath,
    /// Set whenever the `id` parses, even if another field doesn't, so references to it resolve.
    pub id: Option<DecisionId>,
    pub record: Option<Record>,
    pub findings: Vec<Finding>,
    /// The frontmatter's top-level keys, so tree-wide problems can point at a field's line.
    pub keys: Vec<Key>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordSelectError<'a> {
    NoMatch,
    /// More than one record has the ID or prefix, sorted by ID.
    Ambiguous(Vec<&'a ParsedRecord>),
}

fn is_lowercase_hex(text: &str) -> bool {
    text.bytes()
        .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

impl DecisionId {
    pub fn parse(text: &str) -> Option<Self> {
        let hex = text.strip_prefix("dc-")?;
        (hex.len() == 7 && is_lowercase_hex(hex)).then(|| Self(text.to_owned()))
    }

    /// A new ID from 4 random bytes, of which the first 7 hex characters are used. The I/O
    /// side supplies the bytes, so this stays pure.
    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        Self(format!("dc-{}", &hex[..7]))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl DecisionIdPrefix {
    pub fn parse(text: &str) -> Option<Self> {
        let hex = text.strip_prefix("dc-")?;
        ((4..=7).contains(&hex.len()) && is_lowercase_hex(hex)).then(|| Self(text.to_owned()))
    }

    pub fn matches(&self, id: &DecisionId) -> bool {
        id.0.starts_with(&self.0)
    }
}

impl Topic {
    pub fn parse(text: &str) -> Option<Self> {
        let word = |word: &str| {
            !word.is_empty()
                && word
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        };
        text.split('-').all(word).then(|| Self(text.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The days in `month` of `year` in the Gregorian calendar, or `None` for no such month.
fn days_in_month(year: u32, month: u32) -> Option<u32> {
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 if leap => Some(29),
        2 => Some(28),
        _ => None,
    }
}

impl Date {
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('-');
        let (year, month, day) = (parts.next()?, parts.next()?, parts.next()?);
        let number = |part: &str, len: usize| {
            let digits = part.len() == len && part.bytes().all(|byte| byte.is_ascii_digit());
            digits.then(|| part.parse::<u32>().ok()).flatten()
        };
        let year = number(year, 4)?;
        let month = number(month, 2)?;
        let day = number(day, 2)?;
        let last_day = days_in_month(year, month)?;
        let valid = (1..=last_day).contains(&day) && parts.next().is_none();
        valid.then(|| Self(text.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl RecordStatus {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "proposed" => Some(Self::Proposed),
            "accepted" => Some(Self::Accepted),
            "superseded" => Some(Self::Superseded),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::Superseded => "superseded",
            Self::Rejected => "rejected",
        }
    }
}

impl fmt::Display for DecisionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for Topic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl ParsedRecord {
    /// The line of `field`'s key, or the opening `---` when the field is absent.
    pub fn line_of(&self, field: Field) -> u32 {
        let key = self.keys.iter().find(|key| key.name == field.name());
        key.map_or(OPENING_LINE, |key| key.line)
    }
}

/// The record files `show` reads to find a record and the chain it supersedes.
pub fn records_to_read(
    config: &TreeConfig,
    tree_root: &RepoPath,
    paths: &BTreeSet<RepoPath>,
) -> Vec<RepoPath> {
    let layout = layout(config, tree_root, paths);
    let records = layout.records.iter();
    records
        .filter_map(|file| tree_root.join(file.as_str()))
        .collect()
}

/// Every record in the tree whose Markdown is in the snapshot.
pub fn read_records(config: &TreeConfig, snapshot: &Snapshot) -> Vec<ParsedRecord> {
    let layout = layout(config, &snapshot.tree_root, &snapshot.paths);
    let records = layout.records.iter();
    records
        .filter_map(|file| read_record_file(file, markdown(snapshot, file)?))
        .collect()
}

/// Reads one file in a decision records folder. `None` when it has no frontmatter, such as
/// a README, which makes it not a record.
pub fn read_record_file(file: &TreePath, text: &str) -> Option<ParsedRecord> {
    let parsed = match frontmatter::split(text) {
        Split::Absent => return None,
        // The scanned `id` keeps the record indexed, so one deleted line doesn't hide it.
        Split::Unclosed { keys } => ParsedRecord {
            file: file.clone(),
            id: scanned_id(&keys),
            record: None,
            findings: vec![Finding {
                file: file.clone(),
                line: Some(OPENING_LINE),
                problem: Problem::UnparsableFrontmatter("no closing `---` line".into()),
            }],
            keys,
        },
        Split::Present(frontmatter) => parse_record(file, &frontmatter),
    };
    Some(parsed)
}

fn parse_record(file: &TreePath, frontmatter: &Frontmatter) -> ParsedRecord {
    let mut reader = Reader {
        file,
        frontmatter,
        findings: Vec::new(),
    };
    let (id, record) = match frontmatter::values(frontmatter.yaml) {
        Ok(values) => fields(&mut reader, &values),
        Err(error) => {
            let problem = Problem::UnparsableFrontmatter(yaml::error_message(&error));
            reader.report(file_line(&error), problem);
            reader.check_quoting(Field::Title);
            (scanned_id(&frontmatter.keys), None)
        }
    };
    let mut findings = reader.findings;
    findings.extend(too_large(file, frontmatter.body));
    ParsedRecord {
        file: file.clone(),
        id,
        record,
        findings,
        keys: frontmatter.keys.clone(),
    }
}

fn scanned_id(keys: &[Key]) -> Option<DecisionId> {
    DecisionId::parse(scanned_text(keys, Field::Id)?)
}

fn fields(
    reader: &mut Reader,
    values: &BTreeMap<String, Value>,
) -> (Option<DecisionId>, Option<Record>) {
    unknown_fields(reader, values);
    let id = parsed(reader, values, Field::Id, DecisionId::parse);
    let node = parsed(reader, values, Field::Node, NodeId::parse);
    let title = reader.quoted_text(Field::Title, values);
    let status = status(reader, values);
    let topic = parsed(reader, values, Field::Topic, Topic::parse);
    let decided_by = reader.optional_text(Field::DecidedBy, value(values, Field::DecidedBy));
    let date = parsed(reader, values, Field::Date, Date::parse);
    let (source, applies_to) = lists(reader, values);
    let supersedes = supersedes(reader, values);
    let record = || {
        Some(Record {
            id: id.clone()?,
            node: node?,
            title: title?,
            status: status?,
            topic: topic?,
            decided_by: decided_by.ok()?,
            date: date?,
            source,
            applies_to,
            supersedes: supersedes.ok()?,
            body: reader.frontmatter.body.to_owned(),
        })
    };
    let record = record();
    (id, record)
}

const RECORD_FIELDS: [Field; 10] = [
    Field::Id,
    Field::Node,
    Field::Title,
    Field::Status,
    Field::Topic,
    Field::DecidedBy,
    Field::Date,
    Field::Source,
    Field::AppliesTo,
    Field::Supersedes,
];

// A warning, as for nodes, so teams can add their own fields and typos still show.
fn unknown_fields(reader: &mut Reader, values: &BTreeMap<String, Value>) {
    let known = |name: &str| RECORD_FIELDS.iter().any(|field| field.name() == name);
    for name in values.keys().filter(|name| !known(name)) {
        let key = reader.frontmatter.key(name);
        let line = key.map_or(OPENING_LINE, |key| key.line);
        reader.report(line, Problem::UnknownField(name.clone()));
    }
}

/// `source` and `applies_to`. `applies_to` is free text, so any item in its list is kept.
fn lists(reader: &mut Reader, values: &BTreeMap<String, Value>) -> (Vec<IssueRef>, Vec<String>) {
    let source = value(values, Field::Source);
    let source = reader.list(
        Field::Source,
        source,
        IssueRef::parse,
        Problem::MalformedIssueRef,
    );
    let applies_to = value(values, Field::AppliesTo);
    let keep = |text: &str| Some(text.to_owned());
    let applies_to = reader.list(Field::AppliesTo, applies_to, keep, Problem::UnknownField);
    (source, applies_to)
}

/// A required text field, parsed by `parse`. Text that doesn't parse is reported as the
/// field's own problem.
fn parsed<T>(
    reader: &mut Reader,
    values: &BTreeMap<String, Value>,
    field: Field,
    parse: fn(&str) -> Option<T>,
) -> Option<T> {
    let text = reader.text(field, value(values, field))?;
    let parsed = parse(text.as_str());
    if parsed.is_none() {
        reader.report_field(field, malformed(field, text.as_str()));
    }
    parsed
}

fn malformed(field: Field, text: &str) -> Problem {
    match field {
        Field::Id | Field::Supersedes => Problem::MalformedDecisionId(text.to_owned()),
        Field::Node => Problem::MalformedId(text.to_owned()),
        _ => Problem::WrongType {
            field,
            expected: field.expected(),
        },
    }
}

fn status(reader: &mut Reader, values: &BTreeMap<String, Value>) -> Option<RecordStatus> {
    let text = reader.text(Field::Status, value(values, Field::Status))?;
    let status = RecordStatus::parse(text.as_str());
    if status.is_none() {
        let problem = Problem::UnknownRecordStatus(text.as_str().to_owned());
        reader.report_field(Field::Status, problem);
    }
    status
}

/// `Err` when the value has the wrong type or isn't a decision ID, which is reported.
fn supersedes(
    reader: &mut Reader,
    values: &BTreeMap<String, Value>,
) -> Result<Option<DecisionId>, ()> {
    let value = value(values, Field::Supersedes);
    let Some(text) = reader.optional_text(Field::Supersedes, value)? else {
        return Ok(None);
    };
    let id = DecisionId::parse(&text);
    if id.is_none() {
        let problem = malformed(Field::Supersedes, &text);
        reader.report_field(Field::Supersedes, problem);
        return Err(());
    }
    Ok(id)
}

// Counted as `context` counts a body, so authors can compare the two.
fn too_large(file: &TreePath, body: &str) -> Option<Finding> {
    let tokens = estimate(body.trim());
    (tokens > RECORD_TOKENS).then(|| Finding {
        file: file.clone(),
        line: None,
        problem: Problem::RecordTooLarge { tokens },
    })
}

/// The rules across records: unique IDs, a `node` that exists and owns the record's folder,
/// a `supersedes` that resolves, and one accepted record per topic within a node.
pub fn record_findings(records: &[ParsedRecord], nodes: &[ParsedNode]) -> Vec<Finding> {
    let mut findings = duplicate_ids(records);
    findings.extend(node_problems(records, nodes));
    findings.extend(missing_predecessors(records));
    findings.extend(accepted_topics_twice(records));
    findings
}

fn finding(record: &ParsedRecord, field: Field, problem: Problem) -> Finding {
    Finding {
        file: record.file.clone(),
        line: Some(record.line_of(field)),
        problem,
    }
}

// Each file in a group names another file in it.
fn each_names_another<'a>(
    groups: impl Iterator<Item = Vec<&'a ParsedRecord>>,
    field: Field,
    problem: impl Fn(TreePath) -> Problem,
) -> Vec<Finding> {
    let problem = &problem;
    let shared = groups.filter(|records| records.len() > 1);
    shared
        .flat_map(|records| {
            let others = records.clone();
            records.into_iter().map(move |record| {
                let other = others.iter().find(|other| other.file != record.file);
                let also_in = other.map(|other| other.file.clone()).unwrap_or_default();
                finding(record, field, problem(also_in))
            })
        })
        .collect()
}

fn duplicate_ids(records: &[ParsedRecord]) -> Vec<Finding> {
    let mut index: BTreeMap<&DecisionId, Vec<&ParsedRecord>> = BTreeMap::new();
    for record in records {
        if let Some(id) = &record.id {
            index.entry(id).or_default().push(record);
        }
    }
    each_names_another(index.into_values(), Field::Id, |also_in| {
        Problem::DuplicateDecisionId { also_in }
    })
}

fn accepted_topics_twice(records: &[ParsedRecord]) -> Vec<Finding> {
    let mut groups: BTreeMap<(&NodeId, &Topic), Vec<&ParsedRecord>> = BTreeMap::new();
    for parsed in records {
        let Some(record) = &parsed.record else {
            continue;
        };
        if record.status == RecordStatus::Accepted {
            let group = groups.entry((&record.node, &record.topic)).or_default();
            group.push(parsed);
        }
    }
    let groups = groups.into_iter();
    groups
        .flat_map(|((_, topic), records)| {
            let problem = |also_in| Problem::AcceptedTopicTwice {
                topic: topic.clone(),
                also_in,
            };
            each_names_another([records].into_iter(), Field::Topic, problem)
        })
        .collect()
}

// A record's node must exist, and must be the nearest node above its `decisions/` folder.
fn node_problems(records: &[ParsedRecord], nodes: &[ParsedNode]) -> Vec<Finding> {
    let folders: BTreeMap<TreePath, &NodeId> = nodes
        .iter()
        .filter_map(|node| Some((node.location.folder(), node.id.as_ref()?)))
        .collect();
    let ids: BTreeSet<&NodeId> = folders.values().copied().collect();
    let parsed = records.iter().filter_map(|p| Some((p, p.record.as_ref()?)));
    parsed
        .filter_map(|(parsed, record)| {
            let node = record.node.clone();
            if !ids.contains(&node) {
                let problem = Problem::RecordNodeNotFound(node);
                return Some(finding(parsed, Field::Node, problem));
            }
            let folder = records_folder(&parsed.file)?.parent()?;
            let owner = nearest_node(&folders, folder);
            (owner != Some(&node)).then(|| {
                let problem = Problem::RecordInWrongFolder { node };
                finding(parsed, Field::Node, problem)
            })
        })
        .collect()
}

fn nearest_node<'a>(
    folders: &BTreeMap<TreePath, &'a NodeId>,
    folder: TreePath,
) -> Option<&'a NodeId> {
    let mut folder = Some(folder);
    while let Some(current) = folder {
        if let Some(id) = folders.get(&current) {
            return Some(id);
        }
        folder = current.parent();
    }
    None
}

fn missing_predecessors(records: &[ParsedRecord]) -> Vec<Finding> {
    let ids: BTreeSet<&DecisionId> = records.iter().filter_map(|r| r.id.as_ref()).collect();
    let parsed = records.iter().filter_map(|p| Some((p, p.record.as_ref()?)));
    parsed
        .filter_map(|(parsed, record)| {
            let older = record.supersedes.as_ref()?;
            (!ids.contains(older)).then(|| {
                let problem = Problem::SupersededRecordNotFound(older.clone());
                finding(parsed, Field::Supersedes, problem)
            })
        })
        .collect()
}

/// The record that `prefix` names.
pub fn select_record<'a>(
    records: &'a [ParsedRecord],
    prefix: &DecisionIdPrefix,
) -> Result<&'a ParsedRecord, RecordSelectError<'a>> {
    let mut matches: Vec<&ParsedRecord> = records
        .iter()
        .filter(|record| record.id.as_ref().is_some_and(|id| prefix.matches(id)))
        .collect();
    matches.sort_by(|a, b| (&a.id, &a.file).cmp(&(&b.id, &b.file)));
    match matches.as_slice() {
        [] => Err(RecordSelectError::NoMatch),
        [record] => Ok(record),
        _ => Err(RecordSelectError::Ambiguous(matches)),
    }
}

/// The first record, in path order, with `id`.
pub fn find_record<'a>(records: &'a [ParsedRecord], id: &DecisionId) -> Option<&'a ParsedRecord> {
    records.iter().find(|record| record.id.as_ref() == Some(id))
}

/// The ID of the first record, in path order, that supersedes `id`.
pub fn superseded_by<'a>(records: &'a [ParsedRecord], id: &DecisionId) -> Option<&'a DecisionId> {
    let parsed = records.iter().filter_map(|parsed| parsed.record.as_ref());
    let mut newer = parsed.filter(|record| record.supersedes.as_ref() == Some(id));
    newer.next().map(|record| &record.id)
}

/// The records `record` supersedes, newest first, each with its file. The chain stops at a
/// record that's missing, doesn't parse, or is already in it, since `check` doesn't detect
/// cycles.
pub fn superseded_chain<'a>(
    records: &'a [ParsedRecord],
    record: &Record,
) -> Vec<(&'a TreePath, &'a Record)> {
    let mut chain = Vec::new();
    let mut seen = BTreeSet::from([record.id.clone()]);
    let mut next = record.supersedes.clone();
    while let Some(id) = next {
        let Some(parsed) = find_record(records, &id) else {
            break;
        };
        let Some(older) = parsed.record.as_ref().filter(|_| seen.insert(id)) else {
            break;
        };
        chain.push((&parsed.file, older));
        next = older.supersedes.clone();
    }
    chain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_dc_and_seven_lowercase_hex_characters() {
        assert!(DecisionId::parse("dc-3f1a8c2").is_some());
        for text in [
            "dc-3f1a8c",
            "dc-3f1a8c20",
            "dc-3F1A8C2",
            "fd-3f1a8c2",
            "3f1a8c2",
            "",
        ] {
            assert_eq!(DecisionId::parse(text), None, "{text}");
        }
    }

    #[test]
    fn accepts_prefixes_of_four_to_seven_hex_characters() {
        let id = DecisionId::parse("dc-3f1a8c2").unwrap();
        for text in ["dc-3f1a", "dc-3f1a8c", "dc-3f1a8c2"] {
            assert!(
                DecisionIdPrefix::parse(text).unwrap().matches(&id),
                "{text}"
            );
        }
        assert!(!DecisionIdPrefix::parse("dc-3f1b").unwrap().matches(&id));
        for text in ["dc-3f1", "dc-3f1a8c20", "dc-3F1A", "fd-3f1a"] {
            assert_eq!(DecisionIdPrefix::parse(text), None, "{text}");
        }
    }

    #[test]
    fn accepts_kebab_case_topics() {
        for text in ["engine", "search-page-size", "v2-api"] {
            assert!(Topic::parse(text).is_some(), "{text}");
        }
        for text in [
            "",
            "Search",
            "search page",
            "search_page",
            "-search",
            "search--page",
            "search-",
        ] {
            assert_eq!(Topic::parse(text), None, "{text}");
        }
    }

    #[test]
    fn accepts_dates_written_year_month_day() {
        assert!(Date::parse("2026-10-28").is_some());
        for text in [
            "28/10/2026",
            "2026-1-28",
            "2026-13-01",
            "2026-00-10",
            "2026-10-32",
            "2026-10-28T10:00",
            "",
        ] {
            assert_eq!(Date::parse(text), None, "{text}");
        }
    }

    #[test]
    fn rejects_days_past_the_end_of_the_month() {
        for text in ["2026-01-31", "2026-04-30", "2028-02-29", "2000-02-29"] {
            assert!(Date::parse(text).is_some(), "{text}");
        }
        for text in ["2026-02-31", "2026-04-31", "2026-02-29", "2100-02-29"] {
            assert_eq!(Date::parse(text), None, "{text}");
        }
    }

    #[test]
    fn formats_random_bytes_as_a_valid_decision_id() {
        let id = DecisionId::from_bytes([0x3f, 0x1a, 0x8c, 0x2b]);

        assert_eq!(id.as_str(), "dc-3f1a8c2");
        assert_eq!(DecisionId::parse(id.as_str()), Some(id));
    }

    #[test]
    fn follows_the_supersedes_chain_and_stops_at_a_cycle() {
        let record = |id: &str, supersedes: &str| {
            let text = format!(
                "---\nid: {id}\nnode: fd-000001\ntitle: \"T\"\nstatus: accepted\ntopic: t\ndate: 2026-10-28\nsupersedes: {supersedes}\n---\n"
            );
            read_record_file(
                &TreePath::new(&format!("decisions/{id}.md")).unwrap(),
                &text,
            )
            .unwrap()
        };
        let records = [
            record("dc-0000003", "dc-0000002"),
            record("dc-0000002", "dc-0000001"),
            record("dc-0000001", "dc-0000003"),
        ];
        let newest = records[0].record.as_ref().unwrap();

        let chain: Vec<&str> = superseded_chain(&records, newest)
            .iter()
            .map(|(_, record)| record.id.as_str())
            .collect();

        assert_eq!(chain, ["dc-0000002", "dc-0000001"]);
    }
}
