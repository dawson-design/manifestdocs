//! `manifestdocs.yaml`, the file that marks a tree's root.

use serde::Deserialize;
use serde_saphyr::{MessageFormatter, Spanned, UserMessageFormatter};
use thiserror::Error;

use crate::paths::TreePath;
use crate::refs::{Host, RepoRef};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeConfig {
    pub name: TreeName,
    pub github: Option<GitHubConfig>,
    pub merge: MergeMode,
    pub statuses: Vec<StatusName>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubConfig {
    pub host: Host,
    pub docs_repo: RepoRef,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeMode {
    #[default]
    Review,
    Proposed,
}

/// The tree's name, which is also the root node's file name without `.md`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeName(String);

/// A configured node status: one word, never `superseded`, which is built in.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StatusName(String);

/// A config that can't be used. There's no tree without one, so it stops the command.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct ConfigError {
    pub line: Option<u32>,
    pub message: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    name: Spanned<String>,
    github: Option<RawGitHub>,
    decisions: Option<RawDecisions>,
    statuses: Option<Spanned<Vec<String>>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGitHub {
    host: Spanned<String>,
    docs_repo: Spanned<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDecisions {
    merge: Option<MergeMode>,
}

/// The file that marks a tree's root.
pub const FILE_NAME: &str = "manifestdocs.yaml";
const DEFAULT_STATUSES: [&str; 2] = ["draft", "approved"];
pub const SUPERSEDED: &str = "superseded";

pub fn parse(text: &str) -> Result<TreeConfig, ConfigError> {
    let raw: RawConfig = serde_saphyr::from_str(text).map_err(yaml_error)?;
    Ok(TreeConfig {
        name: tree_name(&raw.name)?,
        github: raw.github.map(github).transpose()?,
        merge: raw.decisions.and_then(|d| d.merge).unwrap_or_default(),
        statuses: statuses(raw.statuses)?,
    })
}

impl TreeConfig {
    /// `<name>.md` in the tree's root folder.
    pub fn root_node_file(&self) -> TreePath {
        TreePath::new(&format!("{}.md", self.name.0))
            .expect("a tree name is one file-name-safe segment")
    }
}

impl TreeName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl StatusName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn tree_name(name: &Spanned<String>) -> Result<TreeName, ConfigError> {
    let text = name.value.as_str();
    let file_name_safe = !text.is_empty() && !text.starts_with('.') && !text.contains(['/', '\\']);
    if !file_name_safe {
        return Err(at(
            name,
            format!("`name` `{text}` can't be used as a file name"),
        ));
    }
    Ok(TreeName(text.to_owned()))
}

fn github(raw: RawGitHub) -> Result<GitHubConfig, ConfigError> {
    let host = Host::parse(&raw.host.value).ok_or_else(|| {
        at(
            &raw.host,
            format!("`host` `{}` isn't a host name", raw.host.value),
        )
    })?;
    let docs_repo = RepoRef::parse(&raw.docs_repo.value).ok_or_else(|| {
        let message = format!("`docs_repo` `{}` isn't `owner/name`", raw.docs_repo.value);
        at(&raw.docs_repo, message)
    })?;
    Ok(GitHubConfig { host, docs_repo })
}

fn statuses(raw: Option<Spanned<Vec<String>>>) -> Result<Vec<StatusName>, ConfigError> {
    let Some(raw) = raw else {
        return Ok(DEFAULT_STATUSES.map(|s| StatusName(s.to_owned())).to_vec());
    };
    if let Some(bad) = raw
        .value
        .iter()
        .find(|s| s.is_empty() || s.contains(char::is_whitespace))
    {
        return Err(at(&raw, format!("status `{bad}` must be one word")));
    }
    let statuses: Vec<StatusName> = raw
        .value
        .iter()
        .filter(|s| *s != SUPERSEDED)
        .map(|s| StatusName(s.clone()))
        .collect();
    if statuses.is_empty() {
        return Err(at(
            &raw,
            "`statuses` must list at least one status besides `superseded`".into(),
        ));
    }
    Ok(statuses)
}

fn at<T>(spanned: &Spanned<T>, message: String) -> ConfigError {
    ConfigError {
        line: u32::try_from(spanned.referenced.line())
            .ok()
            .filter(|line| *line > 0),
        message,
    }
}

// The user formatter leaves out the location, which `line` already carries, and the
// library's own setting names.
fn yaml_error(error: serde_saphyr::Error) -> ConfigError {
    ConfigError {
        line: error.location().and_then(|l| u32::try_from(l.line()).ok()),
        message: UserMessageFormatter
            .format_message(error.without_snippet())
            .into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status_names(config: &TreeConfig) -> Vec<&str> {
        config.statuses.iter().map(StatusName::as_str).collect()
    }

    #[test]
    fn fills_in_defaults_when_only_the_name_is_given() {
        let config = parse("name: cookbook\n").unwrap();

        assert_eq!(config.name.as_str(), "cookbook");
        assert_eq!(config.github, None);
        assert_eq!(config.merge, MergeMode::Review);
        assert_eq!(status_names(&config), ["draft", "approved"]);
    }

    #[test]
    fn reads_every_setting() {
        let text = "name: manifest-docs\ngithub:\n  host: github.com\n  docs_repo: acme/cookbook-docs\ndecisions:\n  merge: proposed\nstatuses: [draft, live, superseded]\n";
        let config = parse(text).unwrap();

        let github = config.github.as_ref().unwrap();
        assert_eq!(github.host.to_string(), "github.com");
        assert_eq!(github.docs_repo.to_string(), "acme/cookbook-docs");
        assert_eq!(config.merge, MergeMode::Proposed);
        assert_eq!(status_names(&config), ["draft", "live"]);
    }

    #[test]
    fn reads_plain_scalars_that_look_like_other_types_as_text() {
        assert_eq!(parse("name: yes\n").unwrap().name.as_str(), "yes");
        assert_eq!(parse("name: 123\n").unwrap().name.as_str(), "123");
    }

    #[test]
    fn reports_the_line_of_each_config_error() {
        let cases = [
            ("name: x\nstatues: [draft]\n", 2),
            ("name: x\nname: y\n", 2),
            ("name: x\nstatuses: []\n", 2),
            ("name: x\nstatuses: [superseded]\n", 2),
            ("name: x\nstatuses: [in review]\n", 2),
            ("name: ../x\n", 1),
            (
                "name: x\ngithub:\n  host: github.com\n  docs_repo: manifestdocs\n",
                4,
            ),
            (
                "name: x\ngithub:\n  host: https://github.com\n  docs_repo: a/b\n",
                3,
            ),
            ("name: x\ndecisions:\n  merge: sometimes\n", 3),
        ];
        for (text, line) in cases {
            let error = parse(text).unwrap_err();
            assert_eq!(error.line, Some(line), "{text:?}: {}", error.message);
        }
    }

    #[test]
    fn requires_a_name() {
        assert!(parse("statuses: [draft]\n").is_err());
        assert!(parse("").is_err());
    }
}
