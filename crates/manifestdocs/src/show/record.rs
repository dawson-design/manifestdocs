//! `manifestdocs show <dc-id>`: one decision record in full, then the chain it supersedes.

use std::fmt::Write as _;
use std::process::ExitCode;

use anyhow::{Result, anyhow};
use manifestdocs_core::finding::{Finding, Severity};
use manifestdocs_core::paths::TreePath;
use manifestdocs_core::record::{
    DecisionIdPrefix, ParsedRecord, Record, RecordSelectError, find_record, select_record,
    superseded_chain,
};
use manifestdocs_core::tree::Tree;

use super::{Paths, problems, replacement};
use crate::report::write_stdout;

/// Prints the record `prefix` names, or its frontmatter errors with exit code 1.
pub fn run(
    tree: &Tree,
    records: &[ParsedRecord],
    arg: &str,
    prefix: &DecisionIdPrefix,
    paths: &Paths,
) -> Result<ExitCode> {
    let parsed = select_record(records, prefix).map_err(|error| select_error(arg, error, paths))?;
    let errors: Vec<Finding> = parsed
        .findings
        .iter()
        .filter(|finding| finding.problem.severity() == Severity::Error)
        .cloned()
        .collect();
    match &parsed.record {
        Some(record) if errors.is_empty() => {
            write_stdout(&render(tree, records, &parsed.file, record, paths))?;
            Ok(ExitCode::SUCCESS)
        }
        _ => {
            write_stdout(&problems(&errors, paths))?;
            Ok(ExitCode::from(1))
        }
    }
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
