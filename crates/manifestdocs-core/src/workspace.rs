//! The user config, which maps workspace folders to the trees their sessions use.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_saphyr::Spanned;

use crate::config::{ConfigError, at, yaml_error};

/// Where the user config is, relative to the home folder.
pub const FILE_PATH: &str = ".config/manifestdocs/config.yaml";

/// An absolute path on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbsPath(PathBuf);

/// Sessions anywhere under `path` use the tree whose root folder is `tree`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    pub path: AbsPath,
    pub tree: AbsPath,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawUserConfig {
    workspaces: Option<Vec<RawWorkspace>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWorkspace {
    path: Spanned<String>,
    tree: Spanned<String>,
}

impl AbsPath {
    pub fn new(path: PathBuf) -> Option<Self> {
        path.is_absolute().then_some(Self(path))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// Reads the user config, expanding a leading `~/` to `home`.
pub fn parse(text: &str, home: &Path) -> Result<Vec<Workspace>, ConfigError> {
    let raw: Option<RawUserConfig> = serde_saphyr::from_str(text).map_err(yaml_error)?;
    let workspaces = raw.and_then(|raw| raw.workspaces).unwrap_or_default();
    workspaces
        .iter()
        .map(|workspace| {
            Ok(Workspace {
                path: abs_path("path", &workspace.path, home)?,
                tree: abs_path("tree", &workspace.tree, home)?,
            })
        })
        .collect()
}

/// The workspace whose path contains `folder`. When workspaces overlap, the longest path wins.
pub fn workspace_for<'a>(workspaces: &'a [Workspace], folder: &Path) -> Option<&'a Workspace> {
    workspaces
        .iter()
        .filter(|workspace| folder.starts_with(workspace.path.as_path()))
        .max_by_key(|workspace| workspace.path.as_path().components().count())
}

fn abs_path(key: &str, text: &Spanned<String>, home: &Path) -> Result<AbsPath, ConfigError> {
    let value = text.value.as_str();
    let path = match value.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None if value == "~" => home.to_path_buf(),
        None => PathBuf::from(value),
    };
    AbsPath::new(path).ok_or_else(|| {
        let message = format!("workspace `{key}` `{value}` must be absolute or start with `~/`");
        at(text, message)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "/home/sam";

    fn workspaces(text: &str) -> Vec<Workspace> {
        parse(text, Path::new(HOME)).unwrap()
    }

    fn workspace(path: &str, tree: &str) -> Workspace {
        Workspace {
            path: AbsPath::new(PathBuf::from(path)).unwrap(),
            tree: AbsPath::new(PathBuf::from(tree)).unwrap(),
        }
    }

    #[test]
    fn reads_workspaces_and_expands_the_home_folder() {
        let text = "workspaces:\n  - path: ~/work/cookbook\n    tree: ~/work/cookbook/cookbook-docs\n  - path: /srv/acme\n    tree: /srv/acme/docs\n";

        assert_eq!(
            workspaces(text),
            [
                workspace(
                    "/home/sam/work/cookbook",
                    "/home/sam/work/cookbook/cookbook-docs"
                ),
                workspace("/srv/acme", "/srv/acme/docs"),
            ]
        );
    }

    #[test]
    fn reads_a_config_with_no_workspaces() {
        for text in ["", "# nothing yet\n", "workspaces:\n", "workspaces: []\n"] {
            assert_eq!(workspaces(text), [], "{text:?}");
        }
    }

    #[test]
    fn reports_the_line_of_each_config_error() {
        let cases = [
            ("workspace:\n  - path: /a\n    tree: /b\n", 1),
            ("workspaces:\n  - path: work/cookbook\n    tree: /b\n", 2),
            ("workspaces:\n  - path: /a\n    tree: ~cookbook-docs\n", 3),
            ("workspaces:\n  - path: /a\n    tree: /b\n    name: x\n", 4),
        ];
        for (text, line) in cases {
            let error = parse(text, Path::new(HOME)).unwrap_err();
            assert_eq!(error.line, Some(line), "{text:?}: {}", error.message);
        }
        assert!(parse("workspaces:\n  - path: /a\n", Path::new(HOME)).is_err());
    }

    #[test]
    fn picks_the_longest_workspace_that_contains_the_folder() {
        // The longest match comes before a shorter one for `cookbook` and after it for
        // `cook`, so neither file order passes for the wrong reason.
        let all = [
            workspace("/work/cookbook", "/work/cookbook/docs"),
            workspace("/work", "/work/docs"),
            workspace("/work/cook", "/work/cook/docs"),
        ];
        let tree = |folder: &str| {
            workspace_for(&all, Path::new(folder)).map(|w| w.tree.as_path().to_owned())
        };

        assert_eq!(
            tree("/work/cookbook/web/src"),
            Some("/work/cookbook/docs".into())
        );
        assert_eq!(tree("/work/cookbook"), Some("/work/cookbook/docs".into()));
        assert_eq!(tree("/work/cook/app"), Some("/work/cook/docs".into()));
        assert_eq!(tree("/work/other"), Some("/work/docs".into()));
        assert_eq!(tree("/srv"), None);
    }
}
