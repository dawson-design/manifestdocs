//! Agent context: a node and its ancestors in full, the docs it links to and its children,
//! and for an issue, the issue and its parents, trimmed to an approximate token budget.

use crate::check::Snapshot;
use crate::decisions;
use crate::frontmatter;
use crate::link::{LinkTarget, resolve};
use crate::markdown::outline;
use crate::node::NodeLocation;
use crate::paths::{RepoPath, TreePath};
use crate::record::{self, ParsedRecord, Record, RecordStatus, find_record, read_records};
use crate::refs::{IssueRef, RepoRef};
use crate::tokens::{estimate, grouped};
use crate::tree::{Entry, Tree};
use crate::work::{Issue, IssueChain};

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

/// The decision records `context` reads first: those in the `decisions/` folders of the
/// target and its ancestors.
pub fn records_to_read(tree: &Tree, snapshot: &Snapshot, entry: &Entry) -> Vec<RepoPath> {
    let nodes = in_full(tree, entry);
    let in_context = |file: &TreePath| {
        let owner = decisions::owner(tree, file);
        owner.is_some_and(|owner| nodes.iter().any(|node| node.file == owner.file))
    };
    let unread = unread_records(tree, snapshot).into_iter();
    unread
        .filter(|(_, file)| in_context(file))
        .map(|(path, _)| path)
        .collect()
}

/// The decision records to read after the first ones. When a decision that `context` shows
/// supersedes a record that isn't read yet, that's every other record in the tree, since
/// only a record's frontmatter gives its ID. Otherwise none.
pub fn predecessors_to_read(tree: &Tree, snapshot: &Snapshot, entry: &Entry) -> Vec<RepoPath> {
    let records = read_records(&tree.config, snapshot);
    let groups = decisions::grouped(tree, &records, Some(entry));
    let parsed = groups.iter().flat_map(|group| group.records.iter());
    let shown = parsed.filter_map(|parsed| parsed.record.as_ref().filter(|r| is_shown(r)));
    let mut older = shown.filter_map(|record| record.supersedes.as_ref());
    if !older.any(|id| find_record(&records, id).is_none()) {
        return Vec::new();
    }
    let unread = unread_records(tree, snapshot).into_iter();
    unread.map(|(path, _)| path).collect()
}

/// The tree's record files that aren't in the snapshot yet, by repo and tree path.
fn unread_records(tree: &Tree, snapshot: &Snapshot) -> Vec<(RepoPath, TreePath)> {
    let root = &snapshot.tree_root;
    let files = record::records_to_read(&tree.config, root, &snapshot.paths).into_iter();
    files
        .filter(|path| !snapshot.markdown.contains_key(path))
        .filter_map(|path| {
            let file = path.in_tree(root)?;
            Some((path, file))
        })
        .collect()
}

/// Superseded and rejected records are left out; `mfd decisions` lists them.
fn is_shown(record: &Record) -> bool {
    matches!(
        record.status,
        RecordStatus::Accepted | RecordStatus::Proposed
    )
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
    /// The issue the context is for, when it's for an issue that links to `entry`.
    pub issue: Option<&'a IssueChain>,
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

/// The sections around the target, in the order they're printed. Pieces fill the budget in
/// their own order, so a section can be printed after pieces that filled the budget later.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Section {
    Issue,
    ParentWork,
    Ancestors,
    Decisions,
    LinkedDocs,
    LinkedNodes,
    OtherLinks,
    Around,
}

impl Section {
    fn heading(self) -> &'static str {
        match self {
            Section::Issue | Section::Ancestors => "",
            Section::ParentWork => "\n## Parent work\n\n",
            Section::Decisions => "\n## Decisions\n\n",
            Section::LinkedDocs => "\n## Linked docs\n",
            Section::LinkedNodes => "\n## Linked nodes\n\n",
            Section::OtherLinks => "\n## Other links\n\n",
            Section::Around => "\n## Around this feature\n\n",
        }
    }
}

/// The header and the target node, which always go in. Sections about an issue go between.
struct Head {
    header: String,
    target: String,
}

pub fn context(request: &Request) -> String {
    let name = match request.issue {
        Some(chain) => chain.issue.reference.to_string(),
        None => request.entry.id.to_string(),
    };
    let head = Head {
        header: header(&name, request.version),
        target: target(request),
    };
    let pieces = pieces(request);
    let limit = request.budget.map(Budget::characters);
    let mut keep = first_fit(&head, &pieces, limit);
    while let Some(fewer) = smaller(&head, &pieces, &keep, limit, request.budget) {
        keep = fewer;
    }
    render(&head, &pieces, &keep, request.budget)
}

/// Whether each piece fits in what's left of `limit` after the pieces before it.
fn first_fit(head: &Head, pieces: &[Piece], limit: Option<usize>) -> Vec<bool> {
    let mut size = chars(&head.header) + chars(&head.target);
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
    head: &Head,
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

/// The kept pieces by section, and what's left out in fill order.
fn render(head: &Head, pieces: &[Piece], keep: &[bool], budget: Option<Budget>) -> String {
    let mut out = head.header.clone();
    let pieces_kept = pieces.iter().zip(keep).filter(|(_, kept)| **kept);
    let mut kept: Vec<&Piece> = pieces_kept.map(|(piece, _)| piece).collect();
    kept.sort_by_key(|piece| piece.section);
    let (before, after): (Vec<&Piece>, Vec<&Piece>) = kept
        .into_iter()
        .partition(|piece| piece.section < Section::Ancestors);
    push_sections(&mut out, &before);
    out.push_str(&head.target);
    push_sections(&mut out, &after);
    let omitted = omitted(pieces, keep);
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

/// `pieces`, sorted by section, each section under its heading.
fn push_sections(out: &mut String, pieces: &[&Piece]) {
    let mut opened: Option<Section> = None;
    for piece in pieces {
        if opened != Some(piece.section) {
            out.push_str(piece.section.heading());
            opened = Some(piece.section);
        }
        out.push_str(&piece.text);
    }
}

/// Each piece that's left out, with its size, in fill order.
fn omitted(pieces: &[Piece], keep: &[bool]) -> Vec<String> {
    let left_out = pieces.iter().zip(keep).filter(|(_, kept)| !**kept);
    left_out
        .map(|(piece, _)| {
            let tokens = grouped(estimate(&piece.text));
            format!("{} ({tokens} tokens)", piece.name)
        })
        .collect()
}

/// The first line names `name`, the node's ID or the issue, and the docs version.
fn header(name: &str, version: &DocsVersion) -> String {
    if let DocsVersion::Commit {
        reference,
        commit,
        date,
    } = version
    {
        return format!("# Context: {name} (docs {reference} @ {commit}, {date})\n");
    }
    let DocsVersion::WorkingTree {
        branch,
        commit,
        date,
        uncommitted,
        default_branch,
    } = version
    else {
        return format!("# Context: {name} (docs not in git)\n");
    };
    let changes = match uncommitted {
        true => ", uncommitted changes",
        false => "",
    };
    let on = branch.as_deref().unwrap_or("detached HEAD");
    let mut out = format!("# Context: {name} (docs {on} @ {commit}, {date}{changes})\n");
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

// In fill order: the target's decisions, each ancestor from the parent up to the root followed
// by its decisions, linked docs, linked nodes, other links, the issue and its parents, then
// the children. A linked node that's also a child appears once, as a child.
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
    let mut pieces = node_pieces(request);
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
    pieces.extend(request.issue.into_iter().flat_map(issue_pieces));
    let around = children.iter();
    pieces.extend(around.map(|child| summary(request, Section::Around, "Child: ", child)));
    pieces
}

/// The target's decisions, then each ancestor followed by its decisions, so trimming from the
/// end drops the oldest decisions of the farthest ancestors first.
fn node_pieces(request: &Request) -> Vec<Piece> {
    let Request { tree, entry, .. } = request;
    let records = read_records(&tree.config, request.snapshot);
    let groups = decisions::grouped(tree, &records, Some(entry));
    let decisions_of = |node: &Entry| -> Vec<Piece> {
        let group = groups.iter().find(|group| group.node.file == node.file);
        let parsed = group.into_iter().flat_map(|group| group.records.iter());
        parsed
            .filter_map(|parsed| decision_piece(request, &records, parsed))
            .collect()
    };
    let mut pieces = decisions_of(entry);
    for (i, ancestor) in in_full(tree, entry).into_iter().skip(1).enumerate() {
        pieces.push(ancestor_piece(request, ancestor, i == 0));
        pieces.extend(decisions_of(ancestor));
    }
    pieces
}

/// An accepted or proposed decision as one line, or a record with frontmatter errors, whose
/// status is unknown, as its ID and path.
fn decision_piece(
    request: &Request,
    records: &[ParsedRecord],
    parsed: &ParsedRecord,
) -> Option<Piece> {
    let file = request.snapshot.tree_root.join(parsed.file.as_str());
    let name = file.map(|file| (request.shown)(&file)).unwrap_or_default();
    let text = match (&parsed.record, &parsed.id) {
        (Some(record), _) if is_shown(record) => decision_line(records, record),
        (Some(_), _) => return None,
        (None, Some(id)) => format!("- {id} (has problems): {name}\n"),
        (None, None) => format!("- {name} (has problems)\n"),
    };
    Some(Piece {
        section: Section::Decisions,
        text,
        name,
    })
}

/// The decision's ID, status and title, the first sentence of its Why paragraph, and the ID
/// and title of the record it supersedes.
fn decision_line(records: &[ParsedRecord], record: &Record) -> String {
    let title = sentence(record.title.as_str());
    let mut text = format!("- {} ({}): {title}", record.id, record.status.as_str());
    if let Some(why) = first_why_sentence(&record.body) {
        text.push_str(&format!(" {why}"));
    }
    if let Some(older) = &record.supersedes {
        text.push_str(&format!(" Supersedes {older}"));
        // A missing or broken older record, which `check` reports, leaves just its ID.
        let older = find_record(records, older).and_then(|older| older.record.as_ref());
        if let Some(older) = older {
            text.push_str(&format!(": {}", older.title.as_str()));
        }
    }
    text.push('\n');
    text
}

/// `title` ending as a sentence, so the Why sentence can follow it.
fn sentence(title: &str) -> String {
    match title.ends_with(['.', '!', '?']) {
        true => title.to_owned(),
        false => format!("{title}."),
    }
}

/// The first sentence of the paragraph that starts `**Why.**`, or of the next paragraph when
/// the label stands alone. `None` when there's no such paragraph or no reason after it. The
/// paragraph's lines are joined with single spaces.
fn first_why_sentence(body: &str) -> Option<String> {
    let paragraphs = paragraphs(body);
    let at = paragraphs.iter().position(|p| p.starts_with("**Why.**"))?;
    let mut reason = paragraphs[at]["**Why.**".len()..].trim();
    if reason.is_empty() {
        reason = paragraphs.get(at + 1).filter(|next| !is_label(next))?;
    }
    let words: Vec<&str> = reason.split_whitespace().collect();
    let text = words.join(" ");
    Some(first_sentence(&text).to_owned())
}

/// Whether `paragraph` starts with a bold label such as `**Rejected.**`.
fn is_label(paragraph: &str) -> bool {
    let label = paragraph
        .strip_prefix("**")
        .and_then(|rest| rest.split_once("**"));
    label.is_some_and(|(label, _)| label.ends_with('.'))
}

/// Each run of non-blank lines, trimmed and joined with newlines.
fn paragraphs(body: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut lines: Vec<&str> = Vec::new();
    for line in body.lines().chain([""]) {
        let line = line.trim();
        if !line.is_empty() {
            lines.push(line);
        } else if !lines.is_empty() {
            paragraphs.push(lines.join("\n"));
            lines.clear();
        }
    }
    paragraphs
}

/// Words whose full stop doesn't end a sentence.
const ABBREVIATIONS: [&str; 4] = ["e.g.", "i.e.", "etc.", "vs."];

/// Up to the first sentence end, or all of `text`. A sentence ends at `.`, `!` or `?`, or one
/// of them and `)`, outside brackets and parentheses, when the next word starts with a capital
/// or a digit, or there's no next word. A common abbreviation doesn't end one.
fn first_sentence(text: &str) -> &str {
    let mut depth = 0usize;
    let mut previous = ' ';
    for (i, c) in text.char_indices() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
        let after_stop = c == ')' && matches!(previous, '.' | '!' | '?');
        let end = i + c.len_utf8();
        if depth == 0 && (matches!(c, '.' | '!' | '?') || after_stop) && ends_sentence(text, end) {
            return &text[..end];
        }
        previous = c;
    }
    text
}

/// Whether a stop that ends at byte `end` of `text` ends a sentence.
fn ends_sentence(text: &str, end: usize) -> bool {
    let (before, after) = text.split_at(end);
    let word = before
        .rsplit(char::is_whitespace)
        .next()
        .unwrap_or_default();
    let word = word.trim_start_matches(['(', '[']).trim_end_matches(')');
    let word = word.to_lowercase();
    if ABBREVIATIONS.contains(&word.as_str()) {
        return false;
    }
    if after.is_empty() {
        return true;
    }
    let next = after.trim_start().chars().next();
    let spaced = after.starts_with(char::is_whitespace);
    spaced && next.is_none_or(|next| next.is_uppercase() || next.is_ascii_digit())
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

/// The issue in full, then its parents as one line each, from the top down.
fn issue_pieces(chain: &IssueChain) -> Vec<Piece> {
    let issue = &chain.issue;
    let mut text = format!("\n## Issue: {} ({})\n", issue.title, notes(chain, issue));
    push_body(&mut text, &issue.body.replace("\r\n", "\n"));
    let mut pieces = vec![Piece {
        section: Section::Issue,
        text,
        name: issue.reference.to_string(),
    }];
    if !chain.parents.is_empty() {
        pieces.push(Piece {
            section: Section::ParentWork,
            text: parent_work(chain),
            name: "parent work".to_owned(),
        });
    }
    pieces
}

/// Each parent indented under the one above it. A parent `gh` can't read, at the top, says
/// why.
fn parent_work(chain: &IssueChain) -> String {
    let repo = chain.issue.reference.repo();
    let mut out = String::new();
    for (depth, parent) in chain.parents.iter().rev().enumerate() {
        let line = match parent {
            Ok(issue) => format!(
                "{} {} ({})",
                short(&issue.reference, repo),
                issue.title,
                notes(chain, issue)
            ),
            Err((reference, why)) => format!("{}: {why}", short(reference, repo)),
        };
        out.push_str(&format!("{}- {line}\n", "  ".repeat(depth)));
    }
    out
}

/// An issue's type, state and, when it has sub-issues, how many are done, as `work` shows them.
fn notes(chain: &IssueChain, issue: &Issue) -> String {
    let mut notes: Vec<String> = issue.issue_type.iter().cloned().collect();
    notes.push(issue.state.as_str().to_owned());
    if let (done, total @ 1..) = chain.progress(issue) {
        notes.push(format!("{done} of {total} done"));
    }
    notes.join(", ")
}

/// `#number` when the issue is in `repo`, and `owner/name#number` otherwise.
fn short(issue: &IssueRef, repo: &RepoRef) -> String {
    let same = issue
        .repo()
        .to_string()
        .eq_ignore_ascii_case(&repo.to_string());
    match same {
        true => format!("#{}", issue.number()),
        false => issue.to_string(),
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

    #[test]
    fn finds_the_first_sentence_of_the_why_paragraph() {
        let cases = [
            (
                "**Decision.** Twenty.\n\n**Why.** Readers stop at 20. They refine instead.\n",
                Some("Readers stop at 20."),
            ),
            (
                "**Why.** Readers rarely scroll\npast 20 results.\nThey refine.\n",
                Some("Readers rarely scroll past 20 results."),
            ),
            (
                "**Why.**\nIt broke the back button!\n",
                Some("It broke the back button!"),
            ),
            (
                "**Why.** `files.rs` reads blobs\n",
                Some("`files.rs` reads blobs"),
            ),
            ("**Why.** Is it? Yes.\n", Some("Is it?")),
            (
                "\r\n**Why.** Windows lines.\r\n\r\nMore.\r\n",
                Some("Windows lines."),
            ),
        ];
        for (body, expected) in cases {
            assert_eq!(first_why_sentence(body).as_deref(), expected, "{body}");
        }
    }

    #[test]
    fn finds_no_why_sentence_without_a_paragraph_that_starts_why() {
        for body in [
            "",
            "**Decision.** Twenty.\n",
            "**Decision.** Twenty. **Why.** Inline.\n",
            "**Why** without the full stop.\n",
            "**Why.**\n",
            "**Why.**\n\n**Rejected.** Infinite scroll.\n",
        ] {
            assert_eq!(first_why_sentence(body), None, "{body}");
        }
    }

    #[test]
    fn reads_the_next_paragraph_when_the_why_label_stands_alone() {
        let body = "**Why.**\n\nReaders stop at 20. They refine.\n\n**Rejected.** Scroll.\n";

        assert_eq!(
            first_why_sentence(body).as_deref(),
            Some("Readers stop at 20.")
        );
    }

    #[test]
    fn never_ends_a_sentence_inside_brackets_or_parentheses() {
        let cases = [
            (
                "See [Fig. 2](a.md) for it. More.",
                "See [Fig. 2](a.md) for it.",
            ),
            (
                "Per the [docs](https://example.com/a. B) it works. More.",
                "Per the [docs](https://example.com/a. B) it works.",
            ),
            (
                "It failed (see No. 4) twice. More.",
                "It failed (see No. 4) twice.",
            ),
        ];
        for (text, expected) in cases {
            assert_eq!(first_sentence(text), expected, "{text}");
        }
    }

    #[test]
    fn ends_a_sentence_only_before_a_capital_a_digit_or_the_end() {
        let cases = [
            ("Made in the U.S. today. More.", "Made in the U.S. today."),
            ("It stops. 20 more follow.", "It stops."),
            ("It stops! Then more.", "It stops!"),
            ("Version 2.5 is out. More.", "Version 2.5 is out."),
            ("The end.", "The end."),
            ("No stop at all", "No stop at all"),
        ];
        for (text, expected) in cases {
            assert_eq!(first_sentence(text), expected, "{text}");
        }
    }

    #[test]
    fn doesnt_end_a_sentence_after_a_common_abbreviation() {
        let cases = [
            (
                "Tools, e.g. Repomix, agree. More.",
                "Tools, e.g. Repomix, agree.",
            ),
            ("E.g. Repomix agrees. More.", "E.g. Repomix agrees."),
            (
                "One way, i.e. This one, works. More.",
                "One way, i.e. This one, works.",
            ),
            (
                "Files, logs etc. Stay local. More.",
                "Files, logs etc. Stay local.",
            ),
            ("Rust vs. Go was close. More.", "Rust vs. Go was close."),
        ];
        for (text, expected) in cases {
            assert_eq!(first_sentence(text), expected, "{text}");
        }
    }

    #[test]
    fn ends_a_sentence_at_a_stop_before_a_closing_parenthesis() {
        let cases = [
            ("It broke (twice.) Then more.", "It broke (twice.)"),
            ("(It broke.) Then more.", "(It broke.)"),
        ];
        for (text, expected) in cases {
            assert_eq!(first_sentence(text), expected, "{text}");
        }
    }

    #[test]
    fn ends_a_title_as_a_sentence_once() {
        assert_eq!(sentence("Show 20 per page"), "Show 20 per page.");
        assert_eq!(sentence("Show 20 per page."), "Show 20 per page.");
        assert_eq!(sentence("Why 20?"), "Why 20?");
    }
}
