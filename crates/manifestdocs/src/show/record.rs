//! `manifestdocs show <dc-id>`: one decision record in full, then the chain it supersedes.

use std::fmt::Write as _;
use std::process::ExitCode;

use anyhow::{Result, anyhow};
use manifestdocs_core::finding::{Finding, Severity};
use manifestdocs_core::paths::TreePath;
use manifestdocs_core::record::{
    DecisionId, DecisionIdPrefix, ParsedRecord, Record, RecordSelectError, find_record,
    select_record, superseded_by, superseded_chain,
};
use manifestdocs_core::tree::Tree;
use serde::Serialize;

use super::{Paths, problems, replacement};
use crate::json;
use crate::report::write_stdout;

/// Prints the record `prefix` names, or its frontmatter errors with exit code 1.
pub fn run(
    tree: &Tree,
    records: &[ParsedRecord],
    arg: &str,
    prefix: &DecisionIdPrefix,
    paths: &Paths,
    json: bool,
) -> Result<ExitCode> {
    let parsed = select_record(records, prefix).map_err(|error| select_error(arg, error, paths))?;
    let errors: Vec<Finding> = parsed
        .findings
        .iter()
        .filter(|finding| finding.problem.severity() == Severity::Error)
        .cloned()
        .collect();
    let valid = parsed.record.as_ref().filter(|_| errors.is_empty());
    if json {
        json::data("show", data(records, parsed, valid, &errors, paths))?;
    } else {
        let text = match valid {
            Some(record) => render(tree, records, &parsed.file, record, paths),
            None => problems(&errors, paths),
        };
        write_stdout(&text)?;
    }
    Ok(match valid {
        Some(_) => ExitCode::SUCCESS,
        None => ExitCode::from(1),
    })
}

fn select_error(arg: &str, error: RecordSelectError, paths: &Paths) -> anyhow::Error {
    match error {
        RecordSelectError::NoMatch => anyhow!("no decision record `{arg}`"),
        RecordSelectError::Ambiguous(records) => {
            let mut message = format!("`{arg}` matches {} decision records:", records.len());
            for record in records {
                let id = record.id.as_ref().map_or("", |id| id.as_str());
                let file = paths.shown(record.file.as_str());
                let _ = write!(message, "\n  {id}  {file}");
            }
            anyhow!(message)
        }
    }
}

// The record, then each record it supersedes, newest first, separated by blank lines.
fn render(
    tree: &Tree,
    records: &[ParsedRecord],
    file: &TreePath,
    record: &Record,
    paths: &Paths,
) -> String {
    let chain = superseded_chain(records, record);
    let all = [(file, record)].into_iter().chain(chain);
    let shown: Vec<String> = all
        .map(|(file, record)| full(tree, records, file, record, paths))
        .collect();
    shown.join("\n")
}

fn full(
    tree: &Tree,
    records: &[ParsedRecord],
    file: &TreePath,
    record: &Record,
    paths: &Paths,
) -> String {
    let mut out = header(tree, records, file, record, paths);
    let body = record.body.trim_start_matches(['\n', '\r']).trim_end();
    if !body.is_empty() {
        let _ = write!(out, "\n{body}\n");
    }
    out
}

// Lines with nothing to say are left out.
fn header(
    tree: &Tree,
    records: &[ParsedRecord],
    file: &TreePath,
    record: &Record,
    paths: &Paths,
) -> String {
    let (title, id) = (record.title.as_str(), &record.id);
    let mut out = format!("{title} ({id}, {})\n", record.status.as_str());
    let _ = writeln!(out, "File: {}", paths.shown(file.as_str()));
    let _ = writeln!(out, "Node: {}", replacement(tree, &record.node));
    let _ = writeln!(out, "Topic: {}", record.topic);
    let _ = writeln!(out, "Date: {}", record.date.as_str());
    let source: Vec<String> = record.source.iter().map(ToString::to_string).collect();
    let lines = [
        ("Decided by", record.decided_by.iter().cloned().collect()),
        ("Source", source),
        ("Applies to", record.applies_to.clone()),
    ];
    for (label, items) in lines {
        if !items.is_empty() {
            let _ = writeln!(out, "{label}: {}", items.join(", "));
        }
    }
    if let Some(id) = &record.supersedes {
        let older = find_record(records, id).and_then(|parsed| parsed.record.as_ref());
        let named = match older {
            Some(older) => format!("{} ({id})", older.title.as_str()),
            None => id.to_string(),
        };
        let _ = writeln!(out, "Supersedes: {named}");
    }
    out
}

/// The selected record, its frontmatter errors and the records it supersedes.
#[derive(Serialize)]
struct Data<'a> {
    #[serde(flatten)]
    record: JsonRecord<'a>,
    findings: Vec<json::JsonFinding>,
    chain: Vec<JsonRecord<'a>>,
}

/// The fields read from frontmatter are `None` when it has errors.
#[derive(Serialize)]
struct JsonRecord<'a> {
    id: Option<&'a str>,
    node: Option<&'a str>,
    title: Option<&'a str>,
    status: Option<&'a str>,
    topic: Option<&'a str>,
    date: Option<&'a str>,
    decided_by: Option<&'a str>,
    source: Option<Vec<String>>,
    applies_to: Option<&'a [String]>,
    supersedes: Option<&'a str>,
    superseded_by: Option<&'a str>,
    body: Option<&'a str>,
    file: String,
}

/// `valid` is the record when its frontmatter has no errors, as text output shows it. In the
/// chain, each record is superseded by the one before it.
fn data<'a>(
    records: &'a [ParsedRecord],
    parsed: &'a ParsedRecord,
    valid: Option<&'a Record>,
    errors: &[Finding],
    paths: &Paths,
) -> Data<'a> {
    let id = parsed.id.as_ref();
    let newer = id.and_then(|id| superseded_by(records, id));
    let mut chain = Vec::new();
    let mut previous = valid.map(|record| &record.id);
    for (file, older) in valid
        .map(|record| superseded_chain(records, record))
        .unwrap_or_default()
    {
        chain.push(json_record(
            Some(&older.id),
            Some(older),
            previous,
            file,
            paths,
        ));
        previous = Some(&older.id);
    }
    let findings = errors.iter();
    Data {
        record: json_record(id, valid, newer, &parsed.file, paths),
        findings: findings
            .map(|f| json::finding(f, paths.tree_root, paths.cwd))
            .collect(),
        chain,
    }
}

/// `id` is known even when `record`, the frontmatter, has errors.
fn json_record<'a>(
    id: Option<&'a DecisionId>,
    record: Option<&'a Record>,
    newer: Option<&'a DecisionId>,
    file: &TreePath,
    paths: &Paths,
) -> JsonRecord<'a> {
    JsonRecord {
        id: id.map(DecisionId::as_str),
        node: record.map(|r| r.node.as_str()),
        title: record.map(|r| r.title.as_str()),
        status: record.map(|r| r.status.as_str()),
        topic: record.map(|r| r.topic.as_str()),
        date: record.map(|r| r.date.as_str()),
        decided_by: record.and_then(|r| r.decided_by.as_deref()),
        source: record.map(|r| r.source.iter().map(ToString::to_string).collect()),
        applies_to: record.map(|r| r.applies_to.as_slice()),
        supersedes: record.and_then(|r| r.supersedes.as_ref().map(DecisionId::as_str)),
        superseded_by: newer.map(DecisionId::as_str),
        // Trimmed as text output trims it.
        body: record.map(|r| r.body.trim_start_matches(['\n', '\r']).trim_end()),
        file: paths.shown(file.as_str()),
    }
}
