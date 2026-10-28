//! The pointer file, which names the tree for every folder under the folder it's in.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_saphyr::Spanned;

use crate::config::{ConfigError, at, yaml_error};

/// Where a pointer is, relative to the folder it serves.
pub const FILE_PATH: &str = ".manifestdocs/config.yaml";

/// A relative path, which may climb with `..`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelPath(PathBuf);

/// Folders under the pointer's folder use the tree whose root is `tree`, relative to that folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pointer {
    pub tree: RelPath,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPointer {
    tree: Spanned<String>,
}

impl RelPath {
    /// `None` when `path` is empty, absolute or starts with `~`, which the tool doesn't expand.
    pub fn new(path: &str) -> Option<Self> {
        let relative = !path.is_empty() && !path.starts_with('~') && Path::new(path).is_relative();
        relative.then(|| Self(PathBuf::from(path)))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

pub fn parse(text: &str) -> Result<Pointer, ConfigError> {
    let raw: RawPointer = serde_saphyr::from_str(text).map_err(yaml_error)?;
    let value = raw.tree.value.as_str();
    let tree = RelPath::new(value).ok_or_else(|| {
        at(
            &raw.tree,
            format!("`tree` `{value}` must be a relative path"),
        )
    })?;
    Ok(Pointer { tree })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(text: &str) -> PathBuf {
        parse(text).unwrap().tree.as_path().to_owned()
    }

    #[test]
    fn reads_a_tree_below_or_beside_the_folder() {
        assert_eq!(tree("tree: docs\n"), PathBuf::from("docs"));
        assert_eq!(
            tree("tree: ../docs/cookbook\n"),
            PathBuf::from("../docs/cookbook")
        );
    }

    #[test]
    fn reports_the_line_of_each_pointer_error() {
        let cases = [
            ("# docs\ntree: /srv/docs\n", 2),
            ("tree: ~/docs\n", 1),
            ("tree: \"\"\n", 1),
            ("tree: docs\nname: x\n", 2),
        ];
        for (text, line) in cases {
            let error = parse(text).unwrap_err();
            assert_eq!(error.line, Some(line), "{text:?}: {}", error.message);
        }
    }

    #[test]
    fn rejects_a_pointer_without_a_tree() {
        for text in ["", "# nothing yet\n", "tree:\n"] {
            assert!(parse(text).is_err(), "{text:?}");
        }
    }
}
