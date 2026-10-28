# Manifest Docs: data model

These are the types `manifestdocs-core` builds from a tree for M0: the two config files, nodes, the tree and the findings `check` reports. Decision records (M2), work links (M3) and context (M1) extend the model in their milestones. Any serde YAML backend can fill the raw structs these types are parsed from, so the model doesn't depend on the YAML crate.

## Terms

- **Node file:** `<folder>/<folder>.md`, or `<name>.md` in the root folder for the root node.
- **Supporting doc:** any other file a node owns.
- **Reference:** a node ID in frontmatter, such as `superseded_by`. It stays valid when a folder moves.
- **Link:** a relative Markdown path in a body. It breaks when its target moves, and `check` reports it.
- **Finding:** one problem that `check` reports.

## Types

```rust
// manifestdocs.yaml
struct TreeConfig {
    name: TreeName,                  // file-name-safe; names the root node file
    github: Option<GitHubConfig>,    // None: work commands are off
    merge: MergeMode,                // default Review
    statuses: Vec<StatusName>,       // default [draft, approved]; never "superseded"
}
struct GitHubConfig { host: Host, docs_repo: RepoRef }
enum MergeMode { Review, Proposed }

// ~/.config/manifestdocs/config.yaml
struct Workspace { path: AbsPath, tree: AbsPath }

struct NodeId(String);               // "fd-" + exactly 6 lowercase hex
struct NodeIdPrefix(String);         // CLI input only: "fd-" + 4 to 6 hex
enum NodeSelector { Id(NodeIdPrefix), Path(TreePath) }

struct Node {
    id: NodeId,
    location: NodeLocation,          // from where the file is, never from frontmatter
    title: NonEmptyString,
    status: NodeStatus,
    summary: NonEmptyString,
    repos: Vec<RepoRef>,             // owner/repo
    work: Vec<IssueRef>,             // owner/repo#n
    body: String,
}
enum NodeLocation { Root, Folder(TreePath) }
enum NodeStatus { Active(StatusName), Superseded { by: NodeId } }

struct Tree {
    config: TreeConfig,
    nodes: BTreeMap<NodeId, Node>,
    parents: BTreeMap<NodeId, NodeId>,     // no entry: the root node, or top level when there is none
    owners: BTreeMap<TreePath, NodeId>,    // supporting docs; no entry: unowned
    skipped_trees: Vec<TreePath>,
}

enum LinkTarget {
    Local { path: RepoPath, anchor: Option<Anchor> },  // can leave the tree, e.g. to an ADR folder beside it
    External(Url),                                      // never fetched
}

struct Finding { file: TreePath, line: Option<u32>, problem: Problem }
enum Problem {
    UnparsableFrontmatter(String), MissingField(Field), UnquotedField(Field),
    MalformedId(String), DuplicateId { also_in: TreePath }, UnknownStatus(String),
    SupersededWithoutReplacement, ReplacementWithoutSuperseded, ReplacementNotFound(NodeId),
    NodeIdOutsideNodeFile(NodeId), BrokenLink(String), MissingAnchor(String),
    NoRootNode, NestedTreeSkipped, UnknownField(String),
}
impl Problem { fn severity(&self) -> Severity { /* the last three are warnings */ } }
```

Two rules are part of the types. A superseded node can't exist without a replacement, so parsing reports one as `SupersededWithoutReplacement`. A finding's severity comes from its problem, so no finding has the wrong severity.

## Where the I/O happens

1. **Reading:** locate the root, list files with `ignore`, and read each file's path and text.
2. **Checking:** pure functions turn those files into findings. They parse the config, sort the paths into node files, supporting docs and nested trees, parse each node, build the tree and run the checks. Tests build trees from in-memory file lists, with no disk.
3. **Reporting:** render the findings and set the exit code.

ID generation gets its random bytes from the I/O side, so the pure code stays deterministic. An invalid `manifestdocs.yaml` or user config stops the command with an error instead of producing findings, because no tree can be built without it.

## Rules `check` enforces

The types can't express these, so `check` does.

1. Node IDs are unique across the tree.
2. A replacement node exists and isn't the node itself.
3. A node's parent is the nearest ancestor folder that is a node. The root node has no parent.
4. Every supporting doc has exactly one owner: the nearest node above it. `decisions/` folders and nested trees are excluded.
5. A node's status is in `statuses`, or is `superseded`.

## Defaults and edge cases

- `superseded` is built in, so `statuses` lists the others. `statuses` defaults to `[draft, approved]`, and listing `superseded` in it changes nothing.
- `merge` defaults to `review`.
- Stored node IDs are exactly 6 hex characters after `fd-`. Commands accept a prefix of 4 to 6, and a prefix that matches more than one node is an error that lists the matches.
- Unknown node fields are warnings, so teams can add their own fields and typos still show.
- `title` and `summary` are double-quoted. An unquoted `: ` breaks the YAML, an unquoted ` #` silently cuts the value short, and a bare `true` or `null` isn't text.
- `check` validates links in supporting docs as well as in node bodies.
- A folder named `decisions` under a node is its records folder, never a node.
- When workspace paths in the user config overlap, the longest match wins.
- Link paths are repo paths, not tree paths, because a link can leave the tree, such as a link to an ADR folder next to it.
