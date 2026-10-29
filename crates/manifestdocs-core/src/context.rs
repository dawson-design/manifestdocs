//! Agent context: a node and its ancestors in full, the docs it links to and its children,
//! trimmed to an approximate token budget.

use crate::check::Snapshot;
use crate::frontmatter;
use crate::link::{LinkTarget, resolve};
use crate::markdown::outline;
use crate::node::NodeLocation;
use crate::paths::{RepoPath, TreePath};
use crate::tokens::{estimate, grouped};
use crate::tree::{Entry, Tree};

/// Which version of the docs the context was read from, for its header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocsVersion {
    /// The tree isn't in a git repo, or the repo has no commits.
    NotInGit,
    WorkingTree {
        /// `None` when HEAD is detached.
        branch: Option<String>,
        /// The checked-out commit's short SHA.
        commit: String,
        /// The commit's date, as `YYYY-MM-DD`.
        date: String,
        /// Whether the tree or a loaded doc has uncommitted or untracked changes.
        uncommitted: bool,
        /// The branch `origin/HEAD` names, when it's set.
        default_branch: Option<String>,
    },
    /// A commit in the docs clone: `origin/HEAD`, or the ref the command was given.
    Commit {
        /// The ref as read, such as `origin/main`.
        reference: String,
        /// The commit's short SHA.
        commit: String,
        /// The commit's date, as `YYYY-MM-DD`.
        date: String,
    },
}

/// An approximate token budget for the whole context, at least 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget(u32);

impl Budget {
    pub fn new(tokens: u32) -> Option<Self> {
        (tokens > 0).then_some(Self(tokens))
    }

    /// The characters that fit: 3 a token, less 10% headroom for tokenizers that need more.
    fn characters(self) -> usize {
        usize::try_from(self.0).unwrap_or(usize::MAX) / 10 * 9 * 3
    }
}

/// What the target node links to, one hop out, each in order of first appearance.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Links {
    /// Local Markdown files that aren't nodes, which load in full.
    pub docs: Vec<RepoPath>,
    /// Other nodes' files, which appear as one-line summaries. Ancestors aren't here, since
    /// they're in full.
    pub nodes: Vec<TreePath>,
    /// Links that are listed, not loaded: other files, folders and URLs in the node's body,
    /// and the links inside the loaded docs that aren't shown elsewhere.
    pub listed: Vec<Listed>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Listed {
    Path(RepoPath),
    Url(String),
}

/// The Markdown `context` needs beyond the node files: the docs the target links to.
pub fn docs_to_read(tree: &Tree, snapshot: &Snapshot, entry: &Entry) -> Vec<RepoPath> {
    let docs = links(tree, snapshot, entry).docs.into_iter();
    docs.filter(|doc| !snapshot.markdown.contains_key(doc))
        .collect()
}

/// The target's links. Links to the target or an ancestor are left out, since they're in
/// full, and so are broken links, which `check` reports.
pub fn links(tree: &Tree, snapshot: &Snapshot, entry: &Entry) -> Links {
    let full: Vec<TreePath> = in_full(tree, entry)
        .iter()
        .map(|e| e.file.clone())
        .collect();
    let mut links = Links::default();
    let file = snapshot.tree_root.join(entry.file.as_str());
    for target in file.iter().flat_map(|file| body_links(snapshot, file)) {
        add_link(&mut links, tree, snapshot, &full, target);
    }
    let children = tree.children(&entry.file).into_iter();
    let mut shown = full;
    shown.extend(links.nodes.iter().cloned());
    shown.extend(children.map(|child| child.file.clone()));
    for doc in links.docs.clone() {
        for target in body_links(snapshot, &doc) {
            if !shown_elsewhere(tree, snapshot, &links, &shown, &target) {
                push_new(&mut links.listed, listed(snapshot, target));
            }
        }
    }
    links
}

/// The target, then its ancestors from the parent up to the root node.
fn in_full<'a>(tree: &'a Tree, entry: &'a Entry) -> Vec<&'a Entry> {
    let mut nodes = vec![entry];
    let mut next = tree.parent(&entry.file);
    while let Some(parent) = next {
        nodes.push(parent);
        next = tree.parent(&parent.file);
    }
    nodes
}

fn add_link(
    links: &mut Links,
    tree: &Tree,
    snapshot: &Snapshot,
    full: &[TreePath],
    target: LinkTarget,
) {
    if let LinkTarget::Local { path, .. } = &target {
        if let Some(node) = node_at(tree, snapshot, path) {
            push_new(&mut links.nodes, (!full.contains(&node)).then_some(node));
            return;
        }
        if path.as_str().ends_with(".md") && snapshot.paths.contains(path) {
            push_new(&mut links.docs, Some(path.clone()));
            return;
        }
    }
    push_new(&mut links.listed, listed(snapshot, target));
}

// A link inside a loaded doc to a node or doc that's already in the context isn't listed.
fn shown_elsewhere(
    tree: &Tree,
    snapshot: &Snapshot,
    links: &Links,
    shown: &[TreePath],
    target: &LinkTarget,
) -> bool {
    let LinkTarget::Local { path, .. } = target else {
        return false;
    };
    let node = node_at(tree, snapshot, path);
    links.docs.contains(path) || node.is_some_and(|node| shown.contains(&node))
}

fn push_new<T: PartialEq>(items: &mut Vec<T>, item: Option<T>) {
    if let Some(item) = item.filter(|item| !items.contains(item)) {
        items.push(item);
    }
}

/// The node file that a link to a node file or node folder names.
fn node_at(tree: &Tree, snapshot: &Snapshot, path: &RepoPath) -> Option<TreePath> {
    let path = path.in_tree(&snapshot.tree_root)?;
    if tree.entries.contains_key(&path) {
        return Some(path);
    }
    tree.entry_at(&path).map(|entry| entry.file.clone())
}

// A local link that names nothing is broken, and `check` reports it.
fn listed(snapshot: &Snapshot, target: LinkTarget) -> Option<Listed> {
    match target {
        LinkTarget::External(url) => Some(Listed::Url(url)),
        LinkTarget::Local { path, .. } => exists(snapshot, &path).then_some(Listed::Path(path)),
    }
}

fn exists(snapshot: &Snapshot, path: &RepoPath) -> bool {
    let folder = format!("{}/", path.as_str());
    let mut paths = snapshot.paths.iter();
    snapshot.paths.contains(path) || paths.any(|p| p.as_str().starts_with(&folder))
}

fn body_links(snapshot: &Snapshot, file: &RepoPath) -> Vec<LinkTarget> {
    let Some(text) = snapshot.markdown.get(file) else {
        return Vec::new();
    };
    let (body, first_line) = frontmatter::body(text);
    let links = outline(body, first_line).links.into_iter();
    links
        .filter_map(|link| resolve(&link.target, file))
        .collect()
}

/// Everything `context` assembles from. The target's frontmatter must be valid.
pub struct Request<'a> {
    pub tree: &'a Tree,
    pub snapshot: &'a Snapshot,
    pub entry: &'a Entry,
    pub version: &'a DocsVersion,
    pub budget: Option<Budget>,
    /// How output names a repo path, such as relative to the current folder.
    pub shown: &'a dyn Fn(&RepoPath) -> String,
}

/// One piece of the context after the target. Pieces fill the budget in order, and a piece
/// that's left out is named at the end.
struct Piece {
    section: Section,
    text: String,
    name: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Ancestors,
    LinkedDocs,
    LinkedNodes,
    OtherLinks,
    Around,
}

impl Section {
    fn heading(self) -> &'static str {
        match self {
            Section::Ancestors => "",
            Section::LinkedDocs => "\n## Linked docs\n",
            Section::LinkedNodes => "\n## Linked nodes\n\n",
            Section::OtherLinks => "\n## Other links\n\n",
            Section::Around => "\n## Around this feature\n\n",
        }
    }
}

pub fn context(request: &Request) -> String {
    let mut head = header(request.entry, request.version);
    head.push_str(&target(request));
    let pieces = pieces(request);
    let limit = request.budget.map(Budget::characters);
    let mut keep = first_fit(&head, &pieces, limit);
    while let Some(fewer) = smaller(&head, &pieces, &keep, limit, request.budget) {
        keep = fewer;
    }
    render(&head, &pieces, &keep, request.budget)
}

/// Whether each piece fits in what's left of `limit` after the pieces before it.
fn first_fit(head: &str, pieces: &[Piece], limit: Option<usize>) -> Vec<bool> {
    let mut size = chars(head);
    let mut opened: Vec<Section> = Vec::new();
    let mut keep = Vec::new();
    for piece in pieces {
        let heading = match opened.contains(&piece.section) {
            true => 0,
            false => chars(piece.section.heading()),
        };
        let added = heading + chars(&piece.text);
        let fits = limit.is_none_or(|limit| size + added <= limit);
        if fits {
            size += added;
            opened.push(piece.section);
        }
        keep.push(fits);
    }
    keep
}

/// The list of what's left out counts against the budget too. While the output is over the
/// limit, this leaves out one more kept piece: the last one whose removal makes the output
/// smaller. A short link can cost more in the list than kept, so it's passed over.
fn smaller(
    head: &str,
    pieces: &[Piece],
    keep: &[bool],
    limit: Option<usize>,
    budget: Option<Budget>,
) -> Option<Vec<bool>> {
    let size = chars(&render(head, pieces, keep, budget));
    if size <= limit? {
        return None;
    }
    let kept = keep.iter().enumerate().filter(|(_, kept)| **kept);
    let candidates = kept.map(|(i, _)| i).collect::<Vec<_>>().into_iter().rev();
    candidates
        .map(|i| {
            let mut fewer = keep.to_vec();
            fewer[i] = false;
            fewer
        })
        .find(|fewer| chars(&render(head, pieces, fewer, budget)) < size)
}

fn render(head: &str, pieces: &[Piece], keep: &[bool], budget: Option<Budget>) -> String {
    let mut out = head.to_owned();
    let mut opened: Vec<Section> = Vec::new();
    let mut omitted = Vec::new();
    for (piece, kept) in pieces.iter().zip(keep) {
        if !kept {
            let tokens = grouped(estimate(&piece.text));
            omitted.push(format!("{} ({tokens} tokens)", piece.name));
            continue;
        }
        if !opened.contains(&piece.section) {
            out.push_str(piece.section.heading());
            opened.push(piece.section);
        }
        out.push_str(&piece.text);
    }
    out.push('\n');
    if !omitted.is_empty() {
        out.push_str(&format!("Omitted for budget: {}\n", omitted.join(", ")));
    }
    let size = grouped(estimate(&out));
    let of_budget = budget.map(|b| format!(" of {}", grouped(b.0)));
    let of_budget = of_budget.unwrap_or_default();
    out.push_str(&format!("Estimated size: {size}{of_budget} tokens\n"));
    out
}

fn header(entry: &Entry, version: &DocsVersion) -> String {
    if let DocsVersion::Commit {
        reference,
        commit,
        date,
    } = version
    {
        return format!(
            "# Context: {} (docs {reference} @ {commit}, {date})\n",
            entry.id
        );
    }
    let DocsVersion::WorkingTree {
        branch,
        commit,
        date,
        uncommitted,
        default_branch,
    } = version
    else {
        return format!("# Context: {} (docs not in git)\n", entry.id);
    };
    let changes = match uncommitted {
        true => ", uncommitted changes",
        false => "",
    };
    let on = branch.as_deref().unwrap_or("detached HEAD");
    let mut out = format!(
        "# Context: {} (docs {on} @ {commit}, {date}{changes})\n",
        entry.id
    );
    // A detached HEAD has no branch to compare.
    if let (Some(branch), Some(default)) = (branch, default_branch)
        && branch != default
    {
        out.push_str(&format!(
            "\nWarning: the docs clone is on `{branch}`, not its default branch `{default}`.\n"
        ));
    }
    out
}

/// The target node in full, which is always included.
fn target(request: &Request) -> String {
    let entry = request.entry;
    let kind = match entry.location {
        NodeLocation::Root => "Project",
        NodeLocation::Folder(_) => "Feature",
    };
    let mut out = format!(
        "\n## {kind}: {}\nPath: {}\n",
        request.tree.heading(entry),
        folder(request, entry)
    );
    push_body(&mut out, body(request, entry));
    out
}

fn folder(request: &Request, entry: &Entry) -> String {
    let folder = request
        .snapshot
        .tree_root
        .join(entry.location.folder().as_str());
    folder
        .map(|folder| (request.shown)(&folder))
        .unwrap_or_default()
}

/// A node's body. A node with frontmatter errors still has one, read from its file, so its
/// rules still reach the context.
fn body<'a>(request: &'a Request, entry: &'a Entry) -> &'a str {
    if let Ok(node) = &entry.node {
        return &node.body;
    }
    let file = request.snapshot.tree_root.join(entry.file.as_str());
    let text = file.and_then(|file| request.snapshot.markdown.get(&file));
    match text.map(|text| frontmatter::split(text)) {
        Some(frontmatter::Split::Present(frontmatter)) => frontmatter.body,
        _ => text.map_or("", String::as_str),
    }
}

fn push_body(out: &mut String, body: &str) {
    let body = body.trim_start_matches(['\n', '\r']).trim_end();
    if !body.is_empty() {
        out.push_str(&format!("\n{body}\n"));
    }
}

// In fill order: the ancestors from the parent up to the root, linked docs, linked nodes,
// other links, then the children. A linked node that's also a child appears once, as a child.
fn pieces(request: &Request) -> Vec<Piece> {
    let Request {
        tree,
        snapshot,
        entry,
        ..
    } = request;
    let links = links(tree, snapshot, entry);
    let children = tree.children(&entry.file);
    let is_child = |file: &&TreePath| children.iter().any(|child| child.file == **file);
    let ancestors = in_full(tree, entry).into_iter().skip(1).enumerate();
    let mut pieces: Vec<Piece> = ancestors
        .map(|(i, ancestor)| ancestor_piece(request, ancestor, i == 0))
        .collect();
    // A doc that couldn't be read, such as one that isn't UTF-8, is listed with the other
    // links instead.
    let (readable, unreadable): (Vec<&RepoPath>, Vec<&RepoPath>) = links
        .docs
        .iter()
        .partition(|doc| snapshot.markdown.contains_key(*doc));
    pieces.extend(
        readable
            .into_iter()
            .filter_map(|doc| linked_doc(request, doc)),
    );
    let linked_nodes = links.nodes.iter().filter(|file| !is_child(file));
    pieces.extend(linked_nodes.filter_map(|file| {
        let node = tree.entries.get(file)?;
        Some(summary(request, Section::LinkedNodes, "", node))
    }));
    let unlisted = unreadable.into_iter().map(|doc| Listed::Path(doc.clone()));
    let listed: Vec<Listed> = unlisted.chain(links.listed.iter().cloned()).collect();
    pieces.extend(listed.iter().map(|link| other_link(request, link)));
    let around = children.iter();
    pieces.extend(around.map(|child| summary(request, Section::Around, "Child: ", child)));
    pieces
}

fn ancestor_piece(request: &Request, ancestor: &Entry, is_parent: bool) -> Piece {
    let heading = request.tree.heading(ancestor);
    let mut text = match (&ancestor.location, is_parent) {
        (NodeLocation::Root, _) => format!("\n## Project: {heading}\n"),
        (_, true) => format!(
            "\n## Parent: {heading}\nPath: {}\n",
            folder(request, ancestor)
        ),
        (_, false) => format!(
            "\n## Ancestor: {heading}\nPath: {}\n",
            folder(request, ancestor)
        ),
    };
    push_body(&mut text, body(request, ancestor));
    Piece {
        section: Section::Ancestors,
        text,
        name: file_name(request, ancestor),
    }
}

fn linked_doc(request: &Request, doc: &RepoPath) -> Option<Piece> {
    let text = request.snapshot.markdown.get(doc)?;
    let name = (request.shown)(doc);
    Some(Piece {
        section: Section::LinkedDocs,
        text: format!("\n### {name}\n\n{}\n", text.trim()),
        name,
    })
}

fn other_link(request: &Request, link: &Listed) -> Piece {
    let name = match link {
        Listed::Path(path) => (request.shown)(path),
        Listed::Url(url) => url.clone(),
    };
    Piece {
        section: Section::OtherLinks,
        text: format!("- {name}\n"),
        name,
    }
}

/// A node as one line: its heading and, when its frontmatter is valid, its summary.
fn summary(request: &Request, section: Section, label: &str, entry: &Entry) -> Piece {
    let heading = request.tree.heading(entry);
    let text = match &entry.node {
        Ok(node) => format!("- {label}{heading}: {}\n", node.summary.as_str()),
        Err(_) => format!("- {label}{heading}\n"),
    };
    Piece {
        section,
        text,
        name: file_name(request, entry),
    }
}

fn file_name(request: &Request, entry: &Entry) -> String {
    let file = request.snapshot.tree_root.join(entry.file.as_str());
    file.map(|file| (request.shown)(&file)).unwrap_or_default()
}

fn chars(text: &str) -> usize {
    text.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_a_budget_of_at_least_one_token() {
        assert_eq!(Budget::new(0), None);
        assert!(Budget::new(1).is_some());
    }
}
