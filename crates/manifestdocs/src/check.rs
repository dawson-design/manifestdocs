//! `manifestdocs check`: validate the tree around the current folder, or the tree that
//! contains a given file.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow, bail};
use manifestdocs_core::check::{Report, Snapshot, check};
use manifestdocs_core::config::TreeConfig;
use manifestdocs_core::refs::{Host, IssueRef, RepoRef};
use manifestdocs_core::work::{GitHubAnswers, WorkLinks, with_github_findings, work_links};

use crate::files;
use crate::gh::{self, GhError, Lookup};
use crate::json;
use crate::locate::locate;
use crate::report::{relative, render, write_stdout};

/// Whether `check` runs its GitHub rules.
pub enum GitHub {
    Check,
    /// `--offline`: skip them without calling `gh`, and without a note, since it was asked for.
    Skip,
}

/// With `strict`, warnings fail the check as errors do.
pub fn run(path: Option<&str>, strict: bool, json: bool, github: GitHub) -> Result<ExitCode> {
    let cwd = env::current_dir().context("can't read the current folder")?;
    let start = match path {
        Some(path) => start_folder(path, &cwd)?,
        None => cwd.clone(),
    };
    let roots = match (locate(&start), path) {
        (Ok(roots), _) => roots,
        // `locate` fails only after walking up finds no tree, so a file isn't in one, whatever
        // a pointer above it names.
        (Err(_), Some(path)) if !cwd.join(path).is_dir() => bail!("`{path}` isn't in a tree"),
        (Err(error), _) => return Err(error),
    };
    if let Some(path) = path {
        in_tree(path, &cwd, &roots.tree_root)?;
    }
    let config = files::read_config(&roots.tree_root, &cwd)?;
    let mut snapshot = files::snapshot(&roots, &config, &cwd)?;
    files::read_anchor_targets(&mut snapshot, &roots, &config, &cwd)?;
    let report = check(&config, &snapshot);
    let report = match github {
        GitHub::Check => with_github_rules(report, &config, &snapshot),
        GitHub::Skip => report,
    };
    match json {
        true => json::check(&report, &roots.tree_root, &cwd)?,
        false => write_stdout(&render(&report, &roots.tree_root, &cwd))?,
    }
    let failed = report.has_errors() || (strict && !report.findings.is_empty());
    Ok(if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

// `check` doesn't need GitHub. It asks `gh` only when the tree has a `github` block: about
// the issues nodes list in `work`, and for issues with `feature:` lines. When `gh` is missing or logged out, it says the GitHub rules were skipped and checks
// the rest; any other failure skips only the repo or issue it names.
fn with_github_rules(report: Report, config: &TreeConfig, snapshot: &Snapshot) -> Report {
    let (Some(github), Some(links)) = (&config.github, work_links(config, snapshot)) else {
        return report;
    };
    let mut answers = GitHubAnswers::default();
    let asked = check_work(&github.host, &links, &mut answers)
        .and_then(|()| search(&github.host, &links, &mut answers));
    match asked {
        Ok(()) => with_github_findings(report, &links, &answers),
        Err(error) => {
            eprintln!("note: skipped the GitHub checks: {error}");
            report
        }
    }
}

fn check_work(host: &Host, links: &WorkLinks, answers: &mut GitHubAnswers) -> GhResult {
    let mut by_repo: BTreeMap<&RepoRef, BTreeSet<u64>> = BTreeMap::new();
    for item in &links.work {
        let numbers = by_repo.entry(item.issue.repo()).or_default();
        numbers.insert(item.issue.number());
    }
    for (repo, numbers) in by_repo {
        let numbers: Vec<u64> = numbers.into_iter().collect();
        let lookups = match gh::lookup_issues(host, repo, &numbers) {
            Err(GhError::Failed(detail)) => {
                eprintln!(
                    "note: skipped checking the work items in `{repo}`: `gh` failed: {detail}"
                );
                continue;
            }
            lookups => lookups?,
        };
        for (number, lookup) in lookups {
            let issue = format!("{repo}#{number}");
            match lookup {
                Lookup::Found => {}
                Lookup::NotFound => answers.not_found.extend(IssueRef::parse(&issue)),
                Lookup::Failed(detail) => eprintln!("note: skipped checking `{issue}`: {detail}"),
            }
        }
    }
    Ok(())
}

fn search(host: &Host, links: &WorkLinks, answers: &mut GitHubAnswers) -> GhResult {
    for repo in &links.searched_repos {
        match gh::search_issues(host, repo) {
            Ok(found) => {
                if found.total > found.read {
                    eprintln!(
                        "note: the search of `{repo}` for `feature:` lines stopped at {} of {} issues, since GitHub search returns at most 1,000",
                        found.read, found.total
                    );
                }
                answers.searched.extend(found.issues);
            }
            Err(GhError::Failed(detail)) => eprintln!(
                "note: skipped searching `{repo}` for `feature:` lines: `gh` failed: {detail}"
            ),
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

type GhResult = Result<(), GhError>;

/// The folder the tree is looked for from: `path` itself, or the folder a file is in.
fn start_folder(path: &str, cwd: &Path) -> Result<PathBuf> {
    let path =
        fs::canonicalize(cwd.join(path)).map_err(|_| anyhow!("no file or folder `{path}`"))?;
    match path.is_dir() {
        true => Ok(path),
        false => Ok(path.parent().map_or(path.clone(), Path::to_path_buf)),
    }
}

// A pointer can lead to a tree that doesn't contain a file, such as a code repo's README. A
// folder is checked the way running `check` in it would be, so `check .` matches `check`.
fn in_tree(path: &str, cwd: &Path, tree_root: &Path) -> Result<()> {
    let file =
        fs::canonicalize(cwd.join(path)).map_err(|_| anyhow!("no file or folder `{path}`"))?;
    let root = fs::canonicalize(tree_root).context("can't read the tree's folder")?;
    if !file.is_dir() && !file.starts_with(&root) {
        let tree = relative(tree_root, cwd);
        bail!("`{path}` isn't in a tree: the nearest tree is `{tree}`");
    }
    Ok(())
}
