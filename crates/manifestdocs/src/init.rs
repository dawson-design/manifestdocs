//! `manifestdocs init`: start a tree in the current folder.

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow, bail};
use manifestdocs_core::config::{self, TreeConfig, TreeName};
use manifestdocs_core::id::NodeId;
use manifestdocs_core::node::{NodeLocation, read_node_file};
use manifestdocs_core::paths::TreePath;

use crate::report::write_stdout;

pub fn run(title: Option<&str>) -> Result<ExitCode> {
    let cwd = env::current_dir().context("can't read the current folder")?;
    let title = match title {
        Some(title) => title,
        None => folder_name(&cwd)?,
    };
    let name = tree_name(title)?;
    let root_file = format!("{}.md", name.as_str());
    for file in [config::FILE_NAME, root_file.as_str()] {
        // Unlike `exists`, this also sees a symlink whose target is missing.
        if fs::symlink_metadata(cwd.join(file)).is_ok() {
            bail!("{file} already exists");
        }
    }
    // A name such as `null` reads as YAML's null unless it's quoted.
    let config_text = format!("name: \"{}\"\n", name.as_str());
    let config = config::parse(&config_text).map_err(|error| anyhow!(error))?;
    let id = NodeId::from_bytes(random_bytes()?);
    let root_text = root_node(&id, title, &config, &root_file)?;
    write_tree(&cwd, &root_file, &root_text, &config_text)?;
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

fn tree_name(title: &str) -> Result<TreeName> {
    if title.contains(char::is_control) {
        bail!("the title can't contain control characters");
    }
    TreeName::from_title(title).ok_or_else(|| {
        anyhow!("can't make a tree name from `{title}`: give a title with letters or digits")
    })
}

/// The root node's text. It's read back through the node parser first, so `init` never
/// writes a node that `check` rejects or a title that YAML changes.
fn root_node(id: &NodeId, title: &str, config: &TreeConfig, root_file: &str) -> Result<String> {
    let status = config.statuses.first().map_or("draft", |s| s.as_str());
    let quoted = double_quoted(title);
    let summary = double_quoted(&format!("TODO: one sentence about {title}."));
    let text = format!(
        "---\nid: {id}\ntitle: {quoted}\nstatus: {status}\nsummary: {summary}\n---\n\n# {title}\n"
    );
    let file = TreePath::new(root_file).unwrap_or_default();
    let parsed = read_node_file(&file, NodeLocation::Root, &text, config);
    let kept = parsed.is_some_and(|parsed| {
        let same_title = parsed.node.is_some_and(|node| node.title.as_str() == title);
        same_title && parsed.findings.is_empty()
    });
    if !kept {
        bail!("can't write `{title}` as a YAML title");
    }
    Ok(text)
}

// A YAML double-quoted scalar, escaping `\` and `"`. The read-back in `root_node` catches
// any other character YAML can't hold.
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

// The root node goes first, since its longer name is the likelier to fail. If the config
// then fails, the root node is removed, so a failed `init` leaves nothing behind.
fn write_tree(cwd: &Path, root_file: &str, root_text: &str, config_text: &str) -> Result<()> {
    let root = cwd.join(root_file);
    create(&root, root_text)?;
    create(&cwd.join(config::FILE_NAME), config_text).inspect_err(|_| {
        let _ = fs::remove_file(&root);
    })
}

// `create_new` never replaces a file, even one made since the check in `run`. A file this
// made but couldn't write is removed.
fn create(path: &Path, text: &str) -> Result<()> {
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("can't create {name}"))?;
    file.write_all(text.as_bytes()).or_else(|error| {
        let _ = fs::remove_file(path);
        Err(error).with_context(|| format!("can't write {name}"))
    })
}
