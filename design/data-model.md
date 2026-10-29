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
    node_tokens: NonZeroU32,         // default 2,500; `check` warns about a body over it on a node with child nodes
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

// What `context` reads and assembles (M1)
enum DocsVersion {
    NotInGit,
    WorkingTree { branch: Option<String>, commit: String, date: String, uncommitted: bool, default_branch: Option<String> },
    // branch is None when HEAD is detached; uncommitted covers the tree and the loaded docs
}
struct Budget(u32);                    // tokens, at least 1; fills characters / 3, less 10%
struct Links {                         // one hop from the target's body, in order of first appearance
    docs: Vec<RepoPath>,               // local Markdown that isn't a node, loaded in full
    nodes: Vec<TreePath>,              // other nodes' files, as one-line summaries
    listed: Vec<Listed>,               // other files, folders, URLs, and links inside loaded docs
}
enum Listed { Path(RepoPath), Url(String) }
// M1 also adds TreeConfig.node_tokens: u32 (default 2500, at least 1)
// and Problem::NodeTooLarge { tokens: u32, limit: u32 }, a warning

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
    NodeTooLarge { tokens: u32, limit: u32 },
}
impl Problem { fn severity(&self) -> Severity { /* the last five are warnings */ } }
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
- The snapshot lists what GitHub shows: dotfiles are in, files the repo's `.gitignore` files exclude are out, and nested repos and submodules are skipped. A docs tree in a submodule is read as another repo. A tracked file that matches `.gitignore` is in, because the working tree's list adds the files in git's index, and a commit's list is every file in it.

## Decision records (M2)

```rust
struct DecisionId(String);             // "dc-" + exactly 7 lowercase hex
struct DecisionIdPrefix(String);       // CLI input only: "dc-" + 4 to 7 hex, with `show`
enum NodeSelector { Id(NodeIdPrefix), Decision(DecisionIdPrefix), Path(TreePath) }

struct Record {
    id: DecisionId,
    node: NodeId,                      // must be the node whose `decisions/` folder holds the file
    title: NonEmptyString,             // double-quoted, as a node's title
    status: RecordStatus,
    topic: Topic,                      // kebab case; one accepted record per topic within a node
    decided_by: Option<String>,
    date: Date,                        // YYYY-MM-DD
    source: Vec<IssueRef>,             // owner/repo#n, for issues and pull requests
    applies_to: Vec<String>,           // free text for readers; never checked
    supersedes: Option<DecisionId>,
    body: String,
}
enum RecordStatus { Proposed, Accepted, Superseded, Rejected }
struct ParsedRecord {
    file: TreePath,
    id: Option<DecisionId>,            // set whenever `id` parses, so references to it resolve
    record: Option<Record>,
    findings: Vec<Finding>,
    keys: Vec<Key>,                    // the frontmatter's top-level keys, so findings across records point at a field's line
}

// Problem gains, all errors except the last:
//   MalformedDecisionId(String), DuplicateDecisionId { also_in: TreePath },
//   UnknownRecordStatus(String), RecordNodeNotFound(NodeId),
//   RecordInWrongFolder { node: NodeId },      // `node` isn't the node owning its `decisions/` folder
//   SupersededRecordNotFound(DecisionId), AcceptedTopicTwice { topic: Topic, also_in: TreePath },
//   RecordTooLarge { tokens: u32 }             // warning, over a fixed 300
// Field gains the record fields; MissingField, EmptyField, UnquotedField and WrongType cover them.
```

- A record is any Markdown file with frontmatter in a node's `decisions/` folder. A file there with no frontmatter, such as a README, isn't a record and isn't checked. A record's file name, `YYYY-MM-DD-topic.md`, is a convention that `check` doesn't enforce, because frontmatter is the source of truth.
- Records are read from the same snapshot as nodes, so they come from the working tree or a commit as nodes do.

## Where the tree is read from (M2)

```rust
enum DocsSource {
    WorkingTree,                       // inside the docs clone, or the tree is in the working repo
    Commit { reference: String, commit: String, date: String },  // origin/HEAD, or the `--ref` given
}
// DocsVersion gains Commit { reference, commit, date } for the context header.
```

- `check`, `tree` and `show` are authoring tools, so they read the docs clone's working tree wherever they run. `context` and `decisions` feed agents: walking up to `manifestdocs.yaml` means the current folder is inside the docs clone, so they read the working tree, and reaching the tree through a pointer from another repo means they read the docs clone's `origin/HEAD`. `--ref <ref>` on any read command overrides the default, resolved in the docs clone.
- An unset `origin/HEAD` is an error that prints `git -C <docs clone> remote set-head origin --auto`. A tree outside git always reads the working tree, and `--ref` there is an error.
- A commit's snapshot lists paths with `git ls-tree -r -z` and reads Markdown with one `git cat-file --batch`, so the core still gets a `Snapshot` and stays free of I/O.
- Every git command runs with `GIT_NO_LAZY_FETCH=1`, so a partial clone missing an object fails with a message that names the commit. A `.git` that git can't run on, such as when `git` isn't installed or `safe.directory` refuses the clone, is an error, not a tree outside git.

## GitHub (M3)

```rust
// Everything the binary reads through `gh`. It never writes.
// `work` reads this shape. `context <issue>` (B3) and `stale` (B4) add `body: String` and
// `created_at: String` (RFC 3339, for the spec-ref fallback) when they need them.
struct Issue {
    reference: IssueRef,               // owner/repo#n
    title: String,
    state: IssueState,
    issue_type: Option<String>,        // a display label only
    parent: Option<IssueRef>,
    sub_issues: Vec<SubIssue>,         // in GitHub's order
}
enum IssueState { Open, Closed }
enum SubIssue { Issue(IssueRef), Unreadable(Unreadable) }  // GitHub can list a sub-issue it can't return
enum Unreadable { NotFound, Failed(String) }               // why `gh` couldn't read one issue
struct IssueLinks { feature: Option<NodeId>, spec_ref: Option<String> }  // from `feature:` and `spec-ref:` body lines
enum GhError { Missing, Unauthenticated { host: Host }, Failed(String) }

// `work`, in the core
struct IssueKey(String);                                   // an IssueRef compared without case
type Fetched = BTreeMap<IssueKey, Result<Issue, Unreadable>>;
type Parents = BTreeMap<IssueKey, Option<IssueRef>>;       // each parent's parent, up to the top
type Takeovers<'a> = BTreeMap<IssueKey, &'a Entry>;        // issues deeper nodes list, and the node
struct WorkEntry<'a> {
    reference: Option<&'a IssueRef>,                       // `None` for a sub-issue GitHub couldn't return
    issue: Result<&'a Issue, &'a Unreadable>,
    parents: Vec<&'a IssueRef>,                            // nearest first
    done: Option<usize>,                                   // closed sub-issues; `None` when taken over
    listed_by: Option<&'a Entry>,
    sub_issues: Vec<WorkEntry<'a>>,
}

// New `Problem` variants for `check`'s GitHub rules
IssueNotFound(IssueRef),                                  // error, on the node's `work` line
IssueNamesUnknownNode { issue: IssueRef, id: NodeId },    // warning, on manifestdocs.yaml
MalformedFeatureLine { issue: IssueRef, value: String },  // warning, on manifestdocs.yaml
```

- `gh api graphql --hostname <github.host>` fetches issues in batches: one query per repo, with an alias for each issue, split into chunks when a repo has many. It reads each issue's parent, sub-issues and issue type when a command needs them. Sub-issues are read level by level until none are left.
- An issue `gh` can't read, or every issue in a repo it can't read, is `Unreadable`, and doesn't stop the command: `work` marks it and exits 1. A failure of a whole query, such as a rate limit, is `GhError::Failed`.
- `GhError` exits 2, except in `check`, which doesn't need GitHub: it prints a `note:` on stderr, skips the GitHub rules the error affects, and checks the rest. `Missing` says to install `gh`. `Unauthenticated` prints `gh auth login --hostname <host>`. Commands that don't need GitHub never call `gh`.
- `IssueLinks` is parsed in the core from the body text. A line counts only when it starts with `feature:` or `spec-ref:`, outside code blocks.
- Tests put a fake `gh` script first on `PATH`. It answers each request from JSON files in the fixture: issue lookups are keyed by issue reference, and searches by repo and page cursor. It fails on any request it has no file for, so a test can't reach GitHub.
