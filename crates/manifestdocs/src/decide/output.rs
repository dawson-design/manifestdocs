//! What `decide` prints: the paths, the records on a dry run, warnings, or the input's errors.

use std::fmt::Write as _;
use std::process::ExitCode;

use anyhow::Result;
use manifestdocs_core::decide::{CloneWarning, Draft, InputError};
use manifestdocs_core::record::ParsedRecord;
use serde::Serialize;

use super::Args;
use crate::json;
use crate::report::write_stdout;

/// `decide`'s JSON `data`. The record's fields are `None` when nothing was or would be written.
#[derive(Serialize)]
struct Data<'a> {
    /// `written`, `dry_run`, `unchanged` or `invalid`.
    outcome: &'static str,
    id: Option<&'a str>,
    status: Option<&'a str>,
    file: Option<&'a str>,
    record: Option<&'a str>,
    superseded: Option<&'a Superseded>,
    errors: Vec<JsonError<'a>>,
    warnings: Vec<JsonWarning>,
}

/// The record a new one supersedes: its path from the current folder, and its text with
/// `status: superseded`.
#[derive(Serialize)]
pub struct Superseded {
    pub id: String,
    pub file: String,
    pub record: String,
}

/// A finding's fields, plus the frontmatter field and the fix. `file` is `None` for stdin.
#[derive(Serialize)]
struct JsonError<'a> {
    file: Option<&'a str>,
    line: u32,
    field: Option<&'a str>,
    severity: &'static str,
    code: &'static str,
    message: String,
    fix: String,
}

#[derive(Serialize)]
struct JsonWarning {
    code: &'static str,
    message: String,
}

impl Data<'_> {
    fn empty(outcome: &'static str) -> Self {
        Data {
            outcome,
            id: None,
            status: None,
            file: None,
            record: None,
            superseded: None,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

/// Every error with its line and fix, and exit code 1. `source` is `None` for stdin.
pub fn invalid(errors: &[InputError], source: Option<&str>, json: bool) -> Result<ExitCode> {
    if json {
        let errors = errors.iter().map(|error| JsonError {
            file: source,
            line: error.line,
            field: error.field.as_deref(),
            severity: "error",
            code: error.code(),
            message: error.problem.to_string(),
            fix: error.fix(),
        });
        let data = Data {
            errors: errors.collect(),
            ..Data::empty("invalid")
        };
        json::data("decide", data)?;
        return Ok(ExitCode::from(1));
    }
    let source = source.unwrap_or("<stdin>");
    let mut out = String::new();
    for error in errors {
        let (line, problem, fix) = (error.line, &error.problem, error.fix());
        let _ = writeln!(out, "{source}:{line}: error: {problem}\n  fix: {fix}");
    }
    let count = match errors.len() {
        1 => "1 error".to_owned(),
        n => format!("{n} errors"),
    };
    let _ = writeln!(out, "\n{count}: nothing written");
    write_stdout(&out)?;
    Ok(ExitCode::from(1))
}

/// The path of the record already written.
pub fn unchanged(existing: &ParsedRecord, path: &str, json: bool) -> Result<ExitCode> {
    if json {
        let record = existing.record.as_ref();
        let data = Data {
            id: existing.id.as_ref().map(|id| id.as_str()),
            status: record.map(|record| record.status.as_str()),
            file: Some(path),
            ..Data::empty("unchanged")
        };
        json::data("decide", data)?;
    } else {
        write_stdout(&format!("{path}\n"))?;
    }
    Ok(ExitCode::SUCCESS)
}

/// The paths, with both records' text on a dry run, and warnings on stderr.
pub fn new(
    draft: &Draft,
    path: &str,
    older: Option<&Superseded>,
    clone: &[CloneWarning],
    args: &Args,
) -> Result<ExitCode> {
    let long = draft.warnings.iter().map(|w| (w.code(), w.to_string()));
    let clone = clone.iter().map(|w| (w.code(), w.to_string()));
    let warnings: Vec<(&str, String)> = long.chain(clone).collect();
    if args.json {
        let warnings = warnings
            .into_iter()
            .map(|(code, message)| JsonWarning { code, message });
        let data = Data {
            id: Some(draft.record.id.as_str()),
            status: Some(draft.record.status.as_str()),
            file: Some(path),
            record: Some(&draft.text),
            superseded: older,
            warnings: warnings.collect(),
            ..Data::empty(if args.dry_run { "dry_run" } else { "written" })
        };
        json::data("decide", data)?;
        return Ok(ExitCode::SUCCESS);
    }
    for (_, message) in &warnings {
        eprintln!("warning: {message}");
    }
    write_stdout(&text(draft, path, older, args.dry_run))?;
    Ok(ExitCode::SUCCESS)
}

fn text(draft: &Draft, path: &str, older: Option<&Superseded>, dry_run: bool) -> String {
    let mut out = format!("{path}\n");
    if dry_run {
        let _ = write!(out, "\n{}", draft.text);
    }
    if let Some(older) = older {
        if dry_run {
            out.push('\n');
        }
        let _ = writeln!(out, "superseded: {}", older.file);
        if dry_run {
            let _ = write!(out, "\n{}", older.record);
        }
    }
    out
}
