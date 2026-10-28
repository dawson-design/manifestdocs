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

    /// The folder this path is in. The root's folder is the root itself.
    pub fn parent(&self) -> RepoPath {
        let folder = self.0.rsplit_once('/').map_or("", |(folder, _)| folder);
        RepoPath(folder.to_owned())
    }

    /// This path relative to `tree_root`, when it is the root or inside it.
    pub fn in_tree(&self, tree_root: &RepoPath) -> Option<TreePath> {
        within(&self.0, &tree_root.0).map(|rest| TreePath(rest.to_owned()))
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

    /// The folder this path is in, or `None` for the root itself.
    pub fn parent(&self) -> Option<TreePath> {
        if self.0.is_empty() {
            return None;
        }
        let folder = self.0.rsplit_once('/').map_or("", |(folder, _)| folder);
        Some(TreePath(folder.to_owned()))
    }

    /// The last segment, or an empty string for the root.
    pub fn file_name(&self) -> &str {
        self.0.rsplit_once('/').map_or(&self.0, |(_, name)| name)
    }

    /// Whether this path is inside `folder`, not counting `folder` itself.
    pub fn is_within(&self, folder: &TreePath) -> bool {
        within(&self.0, &folder.0).is_some_and(|rest| !rest.is_empty())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn within<'a>(path: &'a str, folder: &str) -> Option<&'a str> {
    if folder.is_empty() {
        return Some(path);
    }
    if path == folder {
        return Some("");
    }
    path.strip_prefix(folder)?.strip_prefix('/')
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

// A leading `/` starts from the root, as GitHub resolves root-relative links.
fn normalise(base: &str, path: &str) -> Option<String> {
    let base = if path.starts_with('/') { "" } else { base };
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
            ("Kitchen Notes/a b.md", "Kitchen Notes/a b.md"),
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
    fn places_repo_paths_inside_a_tree() {
        let tree_root = RepoPath::new("docs/design").unwrap();
        let in_tree = |path: &str| {
            RepoPath::new(path)
                .unwrap()
                .in_tree(&tree_root)
                .map(|p| p.0)
        };

        assert_eq!(in_tree("docs/design/cli/cli.md"), Some("cli/cli.md".into()));
        assert_eq!(in_tree("docs/design"), Some("".into()));
        assert_eq!(in_tree("docs/design-notes/x.md"), None);
        assert_eq!(in_tree("README.md"), None);
        assert_eq!(
            RepoPath::new("README.md")
                .unwrap()
                .in_tree(&RepoPath::default())
                .map(|p| p.0),
            Some("README.md".into())
        );
    }

    #[test]
    fn splits_a_tree_path_into_folder_and_name() {
        let path = TreePath::new("cli/flags/flags.md").unwrap();
        let folder = TreePath::new("cli").unwrap();

        assert_eq!(path.parent().map(|p| p.0), Some("cli/flags".into()));
        assert_eq!(path.file_name(), "flags.md");
        assert!(path.is_within(&folder));
        assert!(!folder.is_within(&folder));
        assert!(!TreePath::new("cli-v2/x.md").unwrap().is_within(&folder));
        assert_eq!(
            TreePath::new("x.md").unwrap().parent().map(|p| p.0),
            Some("".into())
        );
        assert_eq!(TreePath::default().parent(), None);
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
        assert_eq!(joined("/README.md"), Some("README.md".into()));
    }
}
