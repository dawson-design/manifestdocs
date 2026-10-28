//! `manifestdocs init`: start a tree in the current folder.

use std::env;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow, bail};
use manifestdocs_core::config::{self, TreeName};
use manifestdocs_core::id::NodeId;

use crate::report::write_stdout;

pub fn run(title: Option<&str>) -> Result<ExitCode> {
    let cwd = env::current_dir().context("can't read the current folder")?;
    let title = match title {
        Some(title) => title,
        None => folder_name(&cwd)?,
    };
    if title.contains(char::is_control) {
        bail!("the title can't contain control characters");
    }
    let name = TreeName::from_title(title).ok_or_else(|| {
        anyhow!("can't make a tree name from `{title}`: give a title with letters or digits")
    })?;
    let root_file = format!("{}.md", name.as_str());
    for file in [config::FILE_NAME, &root_file] {
        if cwd.join(file).exists() {
            bail!("{file} already exists");
        }
    }
    // A name such as `null` reads as YAML's null unless it's quoted.
    let config_text = format!("name: \"{}\"\n", name.as_str());
    let config = config::parse(&config_text).map_err(|error| anyhow!(error))?;
    let status = config.statuses.first().map_or("draft", |s| s.as_str());
    let id = NodeId::from_bytes(random_bytes()?);
    create(&cwd.join(config::FILE_NAME), &config_text)?;
    create(&cwd.join(&root_file), &root_node(&id, title, status))?;
    write_stdout(&format!(
        "created {} and {root_file} ({id})\n",
        config::FILE_NAME
    ))?;
    Ok(ExitCode::SUCCESS)
}

fn folder_name(folder: &Path) -> Result<&str> {
    folder
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("the current folder's name can't be a title, so give one"))
}

fn root_node(id: &NodeId, title: &str, status: &str) -> String {
    let quoted = double_quoted(title);
    let summary = double_quoted(&format!("TODO: one sentence about {title}."));
    format!(
        "---\nid: {id}\ntitle: {quoted}\nstatus: {status}\nsummary: {summary}\n---\n\n# {title}\n"
    )
}

// A YAML double-quoted scalar. Control characters are rejected before this, so only `\`
// and `"` need escaping.
fn double_quoted(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

// The operating system's random source. The release targets are macOS and Linux, which
// both have /dev/urandom.
fn random_bytes() -> Result<[u8; 3]> {
    let mut bytes = [0; 3];
    File::open("/dev/urandom")
        .and_then(|mut source| source.read_exact(&mut bytes))
        .context("can't read random bytes for the node ID")?;
    Ok(bytes)
}

// `create_new` never replaces a file, even one made since the check above.
fn create(path: &Path, text: &str) -> Result<()> {
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("can't create {name}"))?;
    file.write_all(text.as_bytes())
        .with_context(|| format!("can't write {name}"))
}
