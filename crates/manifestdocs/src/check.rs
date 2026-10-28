//! `manifestdocs check`: validate the tree around the current folder.

use std::env;
use std::process::ExitCode;

use anyhow::{Context, Result};
use manifestdocs_core::check::check;

use crate::files;
use crate::locate::locate;
use crate::report::{render, write_stdout};

pub fn run() -> Result<ExitCode> {
    let cwd = env::current_dir().context("can't read the current folder")?;
    let roots = locate(&cwd)?;
    let config = files::read_config(&roots.tree_root, &cwd)?;
    let mut snapshot = files::snapshot(&roots, &config, &cwd)?;
    files::read_anchor_targets(&mut snapshot, &roots, &config, &cwd)?;
    let report = check(&config, &snapshot);
    write_stdout(&render(&report, &roots.tree_root, &cwd))?;
    Ok(if report.has_errors() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}
