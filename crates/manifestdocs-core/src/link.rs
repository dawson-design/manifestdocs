//! Link targets in Markdown, resolved against the repo's files.

use std::collections::{BTreeMap, BTreeSet};

use percent_encoding::percent_decode_str;

use crate::frontmatter;
use crate::markdown::outline;
use crate::paths::RepoPath;
use crate::slug;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkTarget {
    /// A file or folder in the repo, with the anchor after `#` when there is one.
    Local {
        path: RepoPath,
        anchor: Option<String>,
    },
    /// A URL with a scheme, or a `//host` link. Never parsed or fetched.
    External(String),
}

/// The repo's files, the folders that hold them, and the anchors in the Markdown that was
/// read, for checking that local links resolve.
pub struct Targets<'a> {
    files: &'a BTreeSet<RepoPath>,
    folders: BTreeSet<&'a str>,
    anchors: BTreeMap<&'a RepoPath, BTreeSet<String>>,
}

/// Resolves `target` as written in the Markdown file `from`. `None` when it can't name a
/// path in the repo: it climbs above the root, or its percent-encoding isn't UTF-8.
pub fn resolve(target: &str, from: &RepoPath) -> Option<LinkTarget> {
    if is_external(target) {
        return Some(LinkTarget::External(target.to_owned()));
    }
    let (path, anchor) = match target.split_once('#') {
        Some((path, anchor)) => (path, Some(anchor)),
        None => (target, None),
    };
    let path = path.split_once('?').map_or(path, |(path, _)| path);
    let anchor = match anchor.filter(|anchor| !anchor.is_empty()) {
        Some(anchor) => Some(decode(anchor)?),
        None => None,
    };
    let path = match path {
        "" => from.clone(),
        path => from.parent().join(&decode(path)?)?,
    };
    Some(LinkTarget::Local { path, anchor })
}

impl<'a> Targets<'a> {
    pub fn new(files: &'a BTreeSet<RepoPath>, markdown: &'a BTreeMap<RepoPath, String>) -> Self {
        let folders = files
            .iter()
            .flat_map(|file| folders_of(file.as_str()))
            .collect();
        let anchors = markdown
            .iter()
            .map(|(path, text)| (path, anchors_in(text)))
            .collect();
        Self {
            files,
            folders,
            anchors,
        }
    }

    /// Whether `path` is a file or folder in the repo, matching case exactly as GitHub does.
    pub fn exists(&self, path: &RepoPath) -> bool {
        let path_str = path.as_str();
        path_str.is_empty() || self.files.contains(path) || self.folders.contains(path_str)
    }

    /// Whether `anchor` matches a heading or HTML anchor in `path`. Paths whose Markdown wasn't
    /// read, such as a source file with a `#L10` line anchor, always match.
    pub fn has_anchor(&self, path: &RepoPath, anchor: &str) -> bool {
        let anchors = self.anchors.get(path);
        anchors.is_none_or(|anchors| anchors.contains(anchor))
    }
}

fn anchors_in(text: &str) -> BTreeSet<String> {
    let (body, first_line) = frontmatter::body(text);
    let outline = outline(body, first_line);
    let mut anchors = slug::anchors(outline.headings.iter().map(String::as_str));
    anchors.extend(outline.html_anchors);
    anchors
}

fn folders_of(file: &str) -> impl Iterator<Item = &str> {
    file.match_indices('/').map(move |(end, _)| &file[..end])
}

fn is_external(target: &str) -> bool {
    if target.starts_with("//") {
        return true;
    }
    let Some((scheme, _)) = target.split_once(':') else {
        return false;
    };
    let scheme_chars = scheme
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    scheme.starts_with(|c: char| c.is_ascii_alphabetic()) && scheme_chars
}

fn decode(text: &str) -> Option<String> {
    let decoded = percent_decode_str(text).decode_utf8().ok()?;
    Some(decoded.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(path: &str, anchor: Option<&str>) -> Option<LinkTarget> {
        Some(LinkTarget::Local {
            path: RepoPath::new(path).unwrap(),
            anchor: anchor.map(str::to_owned),
        })
    }

    #[test]
    fn resolves_local_links_from_the_linking_file() {
        let from = RepoPath::new("docs/search/search.md").unwrap();
        let cases = [
            ("ranking.md", local("docs/search/ranking.md", None)),
            (
                "../recipes/recipes.md#steps",
                local("docs/recipes/recipes.md", Some("steps")),
            ),
            (
                "#write-path",
                local("docs/search/search.md", Some("write-path")),
            ),
            ("#", local("docs/search/search.md", None)),
            ("/README.md", local("README.md", None)),
            (
                "Kitchen%20Notes/a%20b.md",
                local("docs/search/Kitchen Notes/a b.md", None),
            ),
            (
                "diagram.svg?raw=true",
                local("docs/search/diagram.svg", None),
            ),
            ("../../../outside.md", None),
            ("bad%FF.md", None),
        ];
        for (target, expected) in cases {
            assert_eq!(resolve(target, &from), expected, "{target}");
        }
    }

    #[test]
    fn treats_urls_with_a_scheme_or_host_as_external() {
        let from = RepoPath::new("search.md").unwrap();
        for target in [
            "https://example.com/a.md",
            "mailto:sam@example.com",
            "//cdn.example.com/x.png",
        ] {
            assert_eq!(
                resolve(target, &from),
                Some(LinkTarget::External(target.into())),
                "{target}"
            );
        }
    }

    #[test]
    fn finds_files_and_the_folders_that_hold_them() {
        let files: BTreeSet<RepoPath> = ["docs/search/search.md", "README.md"]
            .into_iter()
            .map(|path| RepoPath::new(path).unwrap())
            .collect();
        let markdown = BTreeMap::new();
        let targets = Targets::new(&files, &markdown);
        let exists = |path: &str| targets.exists(&RepoPath::new(path).unwrap());

        assert!(exists("docs/search/search.md"));
        assert!(exists("docs/search"));
        assert!(exists("docs"));
        assert!(exists(""));
        assert!(!exists("docs/Search/search.md"));
        assert!(!exists("docs/sea"));
    }
}
