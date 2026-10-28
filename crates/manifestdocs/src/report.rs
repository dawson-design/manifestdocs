//! Renders findings as `path:line: severity: message` lines and a summary.

use std::fmt::Write;
use std::iter;
use std::path::{Component, Path, PathBuf};

use manifestdocs_core::check::Report;
use manifestdocs_core::finding::{Finding, Problem, Severity};

pub fn render(report: &Report, tree_root: &Path, cwd: &Path) -> String {
    let mut out = String::new();
    for finding in &report.findings {
        let _ = writeln!(out, "{}", finding_line(finding, tree_root, cwd));
    }
    if !report.findings.is_empty() {
        out.push('\n');
    }
    let _ = writeln!(out, "{}", summary(report));
    out
}

/// `path` relative to `base`, the way a shell in `base` would name it.
pub fn relative(path: &Path, base: &Path) -> String {
    let path: Vec<Component> = path.components().collect();
    let base: Vec<Component> = base.components().collect();
    let shared = path.iter().zip(&base).take_while(|(a, b)| a == b).count();
    let ups = iter::repeat_n(Component::ParentDir, base.len() - shared);
    let relative: PathBuf = ups.chain(path[shared..].iter().copied()).collect();
    if relative.as_os_str().is_empty() {
        return ".".to_owned();
    }
    relative.display().to_string()
}

fn finding_line(finding: &Finding, tree_root: &Path, cwd: &Path) -> String {
    let mut path = relative(&tree_root.join(finding.file.as_str()), cwd);
    if finding.problem == Problem::NestedTreeSkipped {
        path.push('/');
    }
    let place = match finding.line {
        Some(line) => format!("{path}:{line}"),
        None => path,
    };
    let problem = &finding.problem;
    format!("{place}: {}: {problem}", problem.severity())
}

fn summary(report: &Report) -> String {
    let count = |severity| {
        let findings = report.findings.iter();
        findings
            .filter(|f| f.problem.severity() == severity)
            .count()
    };
    let counts: Vec<String> = [
        (count(Severity::Error), "error"),
        (count(Severity::Warning), "warning"),
    ]
    .into_iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, word)| plural(n, word))
    .collect();
    let problems = if counts.is_empty() {
        "no problems".to_owned()
    } else {
        counts.join(", ")
    };
    format!("{}: {problems}", plural(report.nodes, "node"))
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}
