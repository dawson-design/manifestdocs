//! `manifestdocs decide`: writes one decision record from a file or stdin, and prints its path.

mod output;

use std::collections::BTreeSet;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context, Result, anyhow};
use manifestdocs_core::decide::{
    CloneWarning, Draft, Filled, Outcome, Setting, clone_warnings, decide, mark_superseded,
};
use manifestdocs_core::paths::TreePath;
use manifestdocs_core::record::{
    Date, DecisionId, ParsedRecord, RecordStatus, read_record_file, read_records,
};

use crate::files::{self, ByDefault};
use crate::git;
use crate::locate::{Roots, locate};
use crate::report::relative;

/// Only for tests: a `YYYY-MM-DD` date that `decide` writes instead of today's.
const TEST_DATE: &str = "MANIFESTDOCS_TEST_DATE";

/// The sandbox folder for drafts, in the code repo, which ignores itself.
const LOCAL_FOLDER: &str = ".manifestdocs/decisions";

pub struct Args<'a> {
    /// A path from the current folder, or `-` for stdin.
    pub file: &'a str,
    pub dry_run: bool,
    pub unattended: bool,
    pub out: Option<&'a str>,
    pub json: bool,
}

/// Where the record goes: its node's `decisions/` folder, or a folder outside the tree.
enum Target {
    Node,
    Folder { path: PathBuf, local: bool },
}

/// The records `decide` compares against: the tree's, then any in the target folder.
struct Known {
    records: Vec<ParsedRecord>,
    /// The file of each record, in the same order.
    paths: Vec<PathBuf>,
}

pub fn run(args: &Args) -> Result<ExitCode> {
    let cwd = env::current_dir().context("can't read the current folder")?;
    let input = read_input(args.file, &cwd)?;
    // `decide` writes into the working tree, so it reads the tree there too.
    let docs = files::open(locate(&cwd)?, None, ByDefault::WorkingTree, &cwd)?;
    let roots = &docs.roots;
    let (tree, mut snapshot) = files::tree_and_snapshot(&docs, &cwd)?;
    files::read_records(&mut snapshot, &docs, &tree.config, &cwd)?;
    let target = target(args, &cwd);
    let known = known(read_records(&tree.config, &snapshot), roots, &target)?;
    let setting = Setting {
        merge: tree.config.merge,
        unattended: args.unattended,
    };
    let filled = Filled {
        id: new_id(&known.records)?,
        date: today()?,
    };
    let outcome = decide(&input, setting, &filled, &tree, &known.records);
    let place = Place {
        cwd: &cwd,
        roots,
        target: &target,
        known: &known,
    };
    respond(outcome, args, &place)
}

struct Place<'a> {
    cwd: &'a Path,
    roots: &'a Roots,
    target: &'a Target,
    known: &'a Known,
}

fn respond(outcome: Outcome, args: &Args, place: &Place) -> Result<ExitCode> {
    let shown = |path: &Path| relative(path, place.cwd);
    match outcome {
        Outcome::Invalid(errors) => {
            let source = (args.file != "-").then_some(args.file);
            output::invalid(&errors, source, args.json)
        }
        Outcome::Unchanged(existing) => {
            let path = place.known.path_of(existing);
            output::unchanged(existing, &shown(&path), args.json)
        }
        Outcome::New(draft) => new(&draft, args, place),
    }
}

// The new record is written before the one it supersedes is marked, so a failed write
// leaves the older record as it was.
fn new(draft: &Draft, args: &Args, place: &Place) -> Result<ExitCode> {
    let shown = |path: &Path| relative(path, place.cwd);
    let path = destination(draft, place.target, place.roots);
    let older = superseded(draft, place)?;
    let warnings = warnings(place.target, place.roots);
    if !args.dry_run {
        write(&path, &draft.text, place.target, &shown(&path))?;
        if let Some((older_path, older)) = &older {
            fs::write(older_path, &older.record)
                .with_context(|| format!("can't mark {} superseded", older.file))?;
        }
    }
    let older = older.map(|(_, older)| older);
    output::new(draft, &shown(&path), older.as_ref(), &warnings, args)
}

/// The record `draft` supersedes, with its `status` line changed to `superseded`. `None`
/// when it supersedes nothing, or a record that's already superseded.
fn superseded(draft: &Draft, place: &Place) -> Result<Option<(PathBuf, output::Superseded)>> {
    let Some(id) = &draft.record.supersedes else {
        return Ok(None);
    };
    let known = place.known;
    let Some(index) = known.records.iter().position(|r| r.id.as_ref() == Some(id)) else {
        return Ok(None);
    };
    let status = known.records[index].record.as_ref().map(|r| r.status);
    if status == Some(RecordStatus::Superseded) {
        return Ok(None);
    }
    let path = known.paths[index].clone();
    let file = relative(&path, place.cwd);
    let text =
        fs::read_to_string(&path).with_context(|| format!("can't read {file} as UTF-8 text"))?;
    let record = mark_superseded(&text)
        .ok_or_else(|| anyhow!("can't mark {file} superseded: it has no `status` line"))?;
    let id = id.to_string();
    Ok(Some((path, output::Superseded { id, file, record })))
}

fn read_input(file: &str, cwd: &Path) -> Result<String> {
    let mut text = String::new();
    if file == "-" {
        io::stdin()
            .read_to_string(&mut text)
            .context("can't read the record from stdin as UTF-8 text")?;
        return Ok(text);
    }
    fs::read_to_string(cwd.join(file)).with_context(|| format!("can't read {file} as UTF-8 text"))
}

// `--out` wins. An unattended run is a factory run, which writes into the code repo
// because the docs mount is read-only.
fn target(args: &Args, cwd: &Path) -> Target {
    if let Some(out) = args.out {
        let path = cwd.join(out);
        return Target::Folder { path, local: false };
    }
    if args.unattended {
        let path = repo_root(cwd).join(LOCAL_FOLDER);
        return Target::Folder { path, local: true };
    }
    Target::Node
}

/// The nearest folder at or above `cwd` with a `.git`, or `cwd` when there's none.
fn repo_root(cwd: &Path) -> &Path {
    let mut folders = cwd.ancestors();
    folders
        .find(|folder| folder.join(".git").exists())
        .unwrap_or(cwd)
}

fn known(tree_records: Vec<ParsedRecord>, roots: &Roots, target: &Target) -> Result<Known> {
    let paths = tree_records
        .iter()
        .map(|r| roots.tree_root.join(r.file.as_str()));
    let mut known = Known {
        paths: paths.collect(),
        records: tree_records,
    };
    if let Target::Folder { path, .. } = target {
        for (file, record) in folder_records(path)? {
            known.paths.push(file);
            known.records.push(record);
        }
    }
    Ok(known)
}

/// The records already in a folder outside the tree, such as drafts from earlier in a run.
fn folder_records(folder: &Path) -> Result<Vec<(PathBuf, ParsedRecord)>> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Ok(Vec::new());
    };
    let mut records = Vec::new();
    for entry in entries {
        let path = entry?.path();
        let name = path.file_name().and_then(|name| name.to_str());
        let Some(file) = name.filter(|n| n.ends_with(".md")).and_then(TreePath::new) else {
            continue;
        };
        let text = fs::read_to_string(&path)
            .with_context(|| format!("can't read {} as UTF-8 text", path.display()))?;
        if let Some(record) = read_record_file(&file, &text) {
            records.push((path, record));
        }
    }
    Ok(records)
}

impl Known {
    fn path_of(&self, record: &ParsedRecord) -> PathBuf {
        let index = self.records.iter().position(|r| std::ptr::eq(r, record));
        index.map(|i| self.paths[i].clone()).unwrap_or_default()
    }
}

fn new_id(records: &[ParsedRecord]) -> Result<DecisionId> {
    let taken: BTreeSet<&DecisionId> = records.iter().filter_map(|r| r.id.as_ref()).collect();
    loop {
        let id = DecisionId::from_bytes(random_bytes()?);
        if !taken.contains(&id) {
            return Ok(id);
        }
    }
}

// The operating system's random source, as `init` reads it for node IDs.
fn random_bytes() -> Result<[u8; 4]> {
    let mut bytes = [0; 4];
    File::open("/dev/urandom")
        .and_then(|mut source| source.read_exact(&mut bytes))
        .context("can't read random bytes for the decision ID")?;
    Ok(bytes)
}

/// Today's date in the local time zone, as `date` gives it, or the test date when it's set.
/// The release targets, macOS and Linux, both have `date`.
fn today() -> Result<Date> {
    let text = match env::var(TEST_DATE) {
        Ok(text) => text,
        Err(_) => {
            let output = Command::new("date").arg("+%Y-%m-%d").output();
            let output = output.context("can't run `date` for today's date")?;
            String::from_utf8_lossy(&output.stdout).trim().to_owned()
        }
    };
    Date::parse(&text).ok_or_else(|| anyhow!("today's date `{text}` isn't written YYYY-MM-DD"))
}

fn destination(draft: &Draft, target: &Target, roots: &Roots) -> PathBuf {
    let folder = match target {
        Target::Node => roots.tree_root.join(draft.folder.as_str()),
        Target::Folder { path, .. } => path.clone(),
    };
    // The first free name, so a record that supersedes one from the same day sits beside it.
    let mut paths = (1..).map(|n| folder.join(draft.file_name(n)));
    paths
        .find(|path| fs::symlink_metadata(path).is_err())
        .unwrap_or_default()
}

// Only a write into the docs clone warns about the clone's state.
fn warnings(target: &Target, roots: &Roots) -> Vec<CloneWarning> {
    match target {
        Target::Node => git::clone_state(&roots.tree_root)
            .map(|state| clone_warnings(&state))
            .unwrap_or_default(),
        Target::Folder { .. } => Vec::new(),
    }
}

// A local drafts folder that this creates gets a `.gitignore`, so drafts never get committed
// to the code repo. `create_new` never replaces a file, even one made since `destination`
// found the name free.
fn write(path: &Path, text: &str, target: &Target, shown: &str) -> Result<()> {
    let folder = path.parent().context("the record's path has no folder")?;
    let ignore = matches!(target, Target::Folder { local: true, .. }) && !folder.exists();
    fs::create_dir_all(folder).with_context(|| format!("can't create {}", folder.display()))?;
    if ignore {
        fs::write(folder.join(".gitignore"), "*\n")
            .context("can't write the drafts' .gitignore")?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("can't create {shown}"))?;
    file.write_all(text.as_bytes())
        .with_context(|| format!("can't write {shown}"))
}
