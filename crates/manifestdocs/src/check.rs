//! `manifestdocs check`: validate the tree around the current folder.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow};
use manifestdocs_core::check::check;
use manifestdocs_core::config::{self, TreeConfig};

use crate::files;
use crate::locate::locate;
use crate::report::{relative, render};

pub fn run() -> Result<ExitCode> {
    let cwd = env::current_dir().context("can't read the current folder")?;
    let roots = locate(&cwd)?;
    let config = read_config(&roots.tree_root, &cwd)?;
    let snapshot = files::snapshot(&roots, &config, &cwd)?;
    let report = check(&config, &snapshot);
    write_stdout(&render(&report, &roots.tree_root, &cwd))?;
    Ok(if report.has_errors() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn read_config(tree_root: &Path, cwd: &Path) -> Result<TreeConfig> {
    let path = tree_root.join(config::FILE_NAME);
    let shown = relative(&path, cwd);
    let text = fs::read_to_string(&path).with_context(|| format!("can't read {shown}"))?;
    config::parse(&text).map_err(|error| match error.line {
        Some(line) => anyhow!("{shown}:{line}: {}", error.message),
        None => anyhow!("{shown}: {}", error.message),
    })
}

// A reader such as `head` that closes the pipe early isn't an error.
fn write_stdout(text: &str) -> Result<()> {
    match io::stdout().lock().write_all(text.as_bytes()) {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => {
            Err(error).context("can't write the report")
        }
        _ => Ok(()),
    }
}
