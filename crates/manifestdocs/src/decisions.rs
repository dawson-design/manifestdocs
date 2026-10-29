//! `manifestdocs decisions`: the decision records of a node and its ancestors, or of the
//! whole tree, one line each, grouped by node.

use std::env;
use std::fmt::Write as _;
use std::process::ExitCode;

use anyhow::{Context, Result};
use manifestdocs_core::decisions::{Group, grouped};
use manifestdocs_core::record::{ParsedRecord, read_records};
use manifestdocs_core::tree::Tree;
use serde::Serialize;

use crate::files::{self, ByDefault};
use crate::json;
use crate::locate::locate;
use crate::report::{relative, write_stdout};
use crate::show::{Paths, select_node};

#[derive(Serialize)]
struct Data<'a> {
    records: Vec<JsonRecord<'a>>,
}

/// The fields read from frontmatter are `None` when it has errors. `node` is the node whose
/// `decisions/` folder holds the record.
#[derive(Serialize)]
struct JsonRecord<'a> {
    id: Option<&'a str>,
    node: &'a str,
    title: Option<&'a str>,
    status: Option<&'a str>,
    topic: Option<&'a str>,
    date: Option<&'a str>,
    supersedes: Option<&'a str>,
    file: String,
}

pub fn run(node: Option<&str>, json: bool, reference: Option<&str>) -> Result<ExitCode> {
    let cwd = env::current_dir().context("can't read the current folder")?;
    // Like `context`, from another repo it lists only merged records.
    let docs = files::open(locate(&cwd)?, reference, ByDefault::DefaultBranch, &cwd)?;
    let (tree, mut snapshot) = files::tree_and_snapshot(&docs, &cwd)?;
    let paths = Paths {
        tree_root: &docs.roots.tree_root,
        cwd: &cwd,
    };
    let entry = match node {
        Some(arg) => Some(select_node(&tree, &snapshot, arg, &paths)?),
        None => None,
    };
    files::read_records(&mut snapshot, &docs, &tree.config, &cwd)?;
    let records = read_records(&tree.config, &snapshot);
    let groups = grouped(&tree, &records, entry);
    if json {
        json::data("decisions", data(&groups, &paths))?;
    } else {
        write_stdout(&text(&tree, &groups, &paths))?;
    }
    Ok(ExitCode::SUCCESS)
}

fn text(tree: &Tree, groups: &[Group], paths: &Paths) -> String {
    if groups.is_empty() {
        return "no decision records\n".to_owned();
    }
    let shown: Vec<String> = groups
        .iter()
        .map(|group| {
            let mut out = format!("{}\n", tree.heading(group.node));
            for record in &group.records {
                let _ = writeln!(out, "  {}", line(record, paths));
            }
            out
        })
        .collect();
    shown.join("\n")
}

// A record with frontmatter errors shows only what's known, as `tree` shows a broken node.
fn line(parsed: &ParsedRecord, paths: &Paths) -> String {
    let file = shown(parsed, paths);
    let id = parsed.id.as_ref().map_or("no ID", |id| id.as_str());
    match &parsed.record {
        Some(record) => {
            let (status, date) = (record.status.as_str(), record.date.as_str());
            format!("{id}  {status}  {date}  {}  {file}", record.title.as_str())
        }
        None => format!("{id}  has problems  {file}"),
    }
}

fn data<'a>(groups: &[Group<'a>], paths: &Paths) -> Data<'a> {
    let records = groups.iter().flat_map(|group| {
        group.records.iter().map(|parsed| {
            let record = parsed.record.as_ref();
            JsonRecord {
                id: parsed.id.as_ref().map(|id| id.as_str()),
                node: group.node.id.as_str(),
                title: record.map(|r| r.title.as_str()),
                status: record.map(|r| r.status.as_str()),
                topic: record.map(|r| r.topic.as_str()),
                date: record.map(|r| r.date.as_str()),
                supersedes: record.and_then(|r| r.supersedes.as_ref().map(|id| id.as_str())),
                file: shown(parsed, paths),
            }
        })
    });
    Data {
        records: records.collect(),
    }
}

fn shown(parsed: &ParsedRecord, paths: &Paths) -> String {
    relative(&paths.tree_root.join(parsed.file.as_str()), paths.cwd)
}
