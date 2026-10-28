//! Paths inside a repo and inside a tree, always `/`-separated and normalised.

use std::fmt;

/// A file or folder path relative to the repo root. Empty for the root itself.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepoPath(String);

/// A file or folder path relative to the tree root. Empty for the root itself.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TreePath(String);

impl RepoPath {
    /// Normalises `path`, or returns `None` when `..` climbs above the root.
    pub fn new(path: &str) -> Option<Self> {
        normalise("", path).map(Self)
    }

    /// Resolves `relative` from this folder, or returns `None` when it climbs above the root.
    pub fn join(&self, relative: &str) -> Option<Self> {
        normalise(&self.0, relative).map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TreePath {
    /// Normalises `path`, or returns `None` when `..` climbs above the root.
    pub fn new(path: &str) -> Option<Self> {
        normalise("", path).map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RepoPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for TreePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn normalise(base: &str, path: &str) -> Option<String> {
    let mut parts: Vec<&str> = base.split('/').filter(|part| !part.is_empty()).collect();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalises_separators_and_dot_segments() {
        let cases = [
            ("", ""),
            ("design/cli/cli.md", "design/cli/cli.md"),
            ("./design//cli/", "design/cli"),
            ("design/cli/../checks/checks.md", "design/checks/checks.md"),
            (
                "Universal Deduplicator/a b.md",
                "Universal Deduplicator/a b.md",
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(
                RepoPath::new(input).map(|p| p.0),
                Some(expected.into()),
                "{input}"
            );
        }
    }

    #[test]
    fn rejects_paths_that_climb_above_the_root() {
        assert_eq!(RepoPath::new(".."), None);
        assert_eq!(RepoPath::new("design/../../x.md"), None);
        assert_eq!(TreePath::new("../x.md"), None);
    }

    #[test]
    fn joins_a_relative_path_from_a_folder() {
        let folder = RepoPath::new("design/cli").unwrap();
        let joined = |relative: &str| folder.join(relative).map(|p| p.0);

        assert_eq!(
            joined("../checks/checks.md"),
            Some("design/checks/checks.md".into())
        );
        assert_eq!(joined("./notes.md"), Some("design/cli/notes.md".into()));
        assert_eq!(joined("../../README.md"), Some("README.md".into()));
        assert_eq!(joined("../../../outside.md"), None);
    }
}
