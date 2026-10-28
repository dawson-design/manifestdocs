# Manifest Docs: data model

These are the types `manifestdocs-core` builds from a tree for M0: the two config files, nodes, the tree and the findings `check` reports. Decision records (M2), work links (M3) and context (M1) extend the model in their milestones. `serde-saphyr` fills the raw structs these types are parsed from, and the tool's own scan of the frontmatter supplies quoting and line numbers, so the model doesn't depend on the YAML crate.

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
    statuses: Vec<StatusName>,       // default [draft, approved]; never "superseded"; never empty
}
struct GitHubConfig { host: Host, docs_repo: RepoRef }
enum MergeMode { Review, Proposed }
struct ConfigError { line: Option<u32>, message: String }   // fatal: no tree without a config

// .manifestdocs/config.yaml
struct Pointer { tree: RelPath }     // relative to the pointer's folder; may climb with `..`

struct NodeId(String);               // "fd-" + exactly 6 lowercase hex
struct NodeIdPrefix(String);         // CLI input only: "fd-" + 4 to 6 hex, with `show`
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

// What `tree` and `show` read
struct Tree {
    config: TreeConfig,
    entries: BTreeMap<TreePath, Entry>,    // keyed by node file, since two files can share an ID
    parents: BTreeMap<TreePath, TreePath>, // node file -> parent's node file; no entry: the root node, or top level when there is none
    owners: BTreeMap<TreePath, TreePath>,  // supporting doc -> owner's node file; no entry: unowned
    nested_trees: Vec<TreePath>,           // a path argument into one is an error that names it
}
struct Entry {                             // a node file whose `id` parses
    file: TreePath,
    location: NodeLocation,
    id: NodeId,
    node: Result<Node, Vec<Finding>>,      // Err: the errors in its frontmatter
}

// What the I/O side reads for the pure core
struct Snapshot {
    tree_root: RepoPath,                   // "" when the tree is the repo root
    paths: BTreeSet<RepoPath>,             // every file in the repo that git doesn't ignore
    markdown: BTreeMap<RepoPath, String>,  // every .md under the tree, plus outside anchor targets
}

enum LinkTarget {
    Local { path: RepoPath, anchor: Option<Anchor> },  // can leave the tree, e.g. to an ADR folder beside it
    External(String),                                   // never parsed or fetched
}

struct Finding { file: TreePath, line: Option<u32>, problem: Problem }   // file can be a folder
enum Problem {
    UnparsableFrontmatter(String), MissingField(Field), EmptyField(Field), UnquotedField(Field),
    WrongType { field: Field, expected: &'static str },
    MalformedId(String), MalformedRepoRef(String), MalformedIssueRef(String),
    DuplicateId { also_in: TreePath }, UnknownStatus(String),
    SupersededWithoutReplacement, ReplacementWithoutSuperseded, SupersededBySelf,
    ReplacementNotFound(NodeId), NodeIdOutsideNodeFile(NodeId),
    BrokenLink(String), MissingAnchor { path: String, anchor: String },
    NoRootNode { expected: TreePath }, NestedTreeSkipped, UnknownField(String), NodeFileWithoutId,
}
impl Problem { fn severity(&self) -> Severity { /* the last four are warnings */ } }
```

Two rules are part of the types. A superseded node can't exist without a replacement, so parsing reports one as `SupersededWithoutReplacement`. A finding's severity comes from its problem, so no finding has the wrong severity.

## Where the I/O happens

1. **Reading:** locate the root by walking up to `manifestdocs.yaml`, stopping at the repo root, then through the nearest pointer at or above the current folder. Parse the config, list the repo's files with `ignore`, and read every Markdown file under the tree.
2. **Checking:** pure functions turn the `Snapshot` into findings. They sort the paths into node files, supporting docs, records and nested trees, parse each node, index the IDs, and check references, links and anchors. Anchors in Markdown outside the tree need one more read: the core names those files, and the I/O side adds them to the `Snapshot`. Tests build a `Snapshot` in memory, with no disk.
3. **Reporting:** render the findings and set the exit code.

ID generation gets its random bytes from the I/O side, so the pure code stays deterministic.

## Rules `check` enforces

The types can't express these, so `check` does.

1. Node IDs are unique across the tree.
2. A replacement node exists and isn't the node itself.
3. A node's status is in `statuses`, or is `superseded`.
4. Local links resolve to a file or folder in the repo, and their anchors match a heading in the target.

## Derived

- A node's parent is the nearest ancestor folder that is a node. The root node has no parent.
- Every supporting doc has exactly one owner: the nearest node above it. `decisions/` folders and nested trees are excluded.

## Defaults and edge cases

- `superseded` is built in, so `statuses` lists the others. `statuses` defaults to `[draft, approved]`, including when it's blank. Listing `superseded` in it, in any case, changes nothing, and an empty list is an error. A status name is one word with no spaces.
- `merge` defaults to `review`.
- An unknown key in `manifestdocs.yaml` is an error, because a typo such as `statues:` would otherwise fall back to the default. An unknown node field is a warning, so teams can add their own fields and typos still show.
- Stored node IDs are exactly 6 hex characters after `fd-`. Commands accept a prefix of 4 to 6, and a prefix that matches more than one node is an error that lists the matches.
- A file in a node file's position with frontmatter but no `id` gets a warning. A file there with no frontmatter is a supporting doc.
- IDs are indexed from every node file whose `id` parses, even when another field is invalid, so one bad field doesn't cause a `ReplacementNotFound` for each node that names it. Links are checked in every body, even when its frontmatter fails.
- `title` and `summary` are double-quoted. An unquoted `: ` breaks the YAML, an unquoted ` #` silently cuts the value short, and a bare `true` or `null` isn't text.
- `check` validates links in supporting docs as well as in node bodies.
- A folder named `decisions` under a node is its records folder, never a node.
- `NoRootNode` is reported on `manifestdocs.yaml` and names the expected root node file. `NestedTreeSkipped` is reported on the nested tree's folder.
- Anchors must match a heading's GitHub slug exactly. Supersession cycles aren't detected.
- A pointer is read only when walking up finds no tree, and the nearest one wins. Its `tree` is required, isn't absolute and doesn't start with `~`, and must lead to a folder with `manifestdocs.yaml`. An unknown key is an error. A `.manifestdocs/` folder without `config.yaml` is skipped.
- `tree` and `show` count a node file whose `id` parses as a node, as `check` does. When its frontmatter has errors, `tree` marks it `has problems`, and `show` prints the errors and exits 1, so one bad edit doesn't hide part of the tree.
- A command-line argument that starts with `fd-` is an ID or ID prefix. A folder with that name is written `./fd-x`.
- A node's supporting docs are files of any type. `manifestdocs.yaml` and decision records aren't supporting docs.
- Link paths are repo paths, not tree paths, because a link can leave the tree, such as a link to an ADR folder next to it.
- The snapshot lists what GitHub shows: dotfiles are in, files the repo's `.gitignore` files exclude are out, and nested repos and submodules are skipped. A tracked file that matches `.gitignore` is missed until M2 reads git's index.
