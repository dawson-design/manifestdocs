# Manifest Docs

Manifest Docs turns a folder of Markdown in a docs repo into a feature tree that people and coding agents can walk. Each feature is a folder with one node file for its requirements and decisions, and every file remains an ordinary Markdown page on GitHub, readable without the tool. The `manifestdocs` binary reads the tree from disk. It has no server or account, and it never writes to GitHub.

## Install

Build from a clone of this repo, with Rust installed:

```sh
cargo install --path crates/manifestdocs
```

This installs two binaries, `manifestdocs` and its short name `mfd`, which behave the same.

## Start a tree

In the docs repo, `init` writes `manifestdocs.yaml` and a root node named after the title:

```console
$ manifestdocs init Cookbook
created manifestdocs.yaml and cookbook.md (fd-59db20)
```

Open `cookbook.md` and replace the placeholder summary.

## Add a feature

A folder is a feature node when it contains a Markdown file with the folder's name, such as `recipes/search/search.md`, with frontmatter like this:

```markdown
---
id: fd-7c2e41
title: "Recipe search"
status: draft
summary: "Finds recipes by ingredient, cuisine and cooking time."
repos: [example-org/cookbook-web]
work: [example-org/cookbook-docs#41]
---

# Recipe search

Results rank by the [ranking rules](ranking-rules.md).
```

- `id` is `fd-` and 6 hex characters, unique in the tree. `openssl rand -hex 3` makes the 6 characters.
- `title` and `summary` are required, and always double-quoted.
- `status` is one of the `statuses` in `manifestdocs.yaml`, which default to `draft` and `approved`. It can also be `superseded`, with `superseded_by` set to the replacement's ID.
- `repos` and `work` are optional lists of code repos and top-level issues.

A node's parent is the nearest folder above it that is a node. Every other file in a node's folder, such as `ranking-rules.md`, is one of its supporting docs. A `decisions/` folder contains decision records and is never a node.

## Give an agent a feature's context

`context` prints what a coding agent reads before planning: the node and its ancestors up to the root node in full, the docs the node links to, and one-line summaries of its children and linked nodes. With `--budget`, it leaves out what doesn't fit and names it by path.

```console
$ manifestdocs context fd-7c2e41 --format md --budget 8000
# Context: fd-7c2e41 (docs main @ 3f9a1c2, 2026-10-27)

## Feature: Recipe search (fd-7c2e41, draft)
Path: recipes/search
...
## Parent: Recipes (fd-1a2b3c, approved)
Path: recipes
...
## Project: Cookbook (fd-59db20, draft)
...
## Linked docs

### recipes/search/ranking-rules.md
...
## Around this feature

- Child: Filters (fd-7c2e99, draft): Narrows results.

Estimated size: 236 of 8,000 tokens
```

## Use it from Claude Code

The plugin in `plugins/manifestdocs` has six skills: `manifestdocs-context`, which loads a node's context before planning, `manifestdocs-author`, which writes valid nodes and proposes spec changes, `manifestdocs-verify`, which checks a change against its node, `manifestdocs-decide`, which drafts decision records for the engineer to confirm and opens their docs PR from a worktree, `manifestdocs-plan-issue`, which drafts issues linked to a node and creates them with `gh` once approved, and `manifestdocs-capture`, which records a decision made in an issue's thread. Its commands follow one workflow: `/manifestdocs:explore`, `/manifestdocs:propose`, `/manifestdocs:apply`, `/manifestdocs:verify` and `/manifestdocs:record`. It also adds an outline of the tree at session start and a `check` after each Markdown edit. `/manifestdocs:record` reviews the session for decisions worth recording, and `/manifestdocs:capture` does the same for an issue's thread. Install it from a clone of this repo:

```sh
claude plugin marketplace add /path/to/manifestdocs
claude plugin install manifestdocs@manifestdocs
```

For agent tools without skills, copy `plugins/manifestdocs/AGENTS-snippet.md` into `AGENTS.md`.

## Read and check the tree

```console
$ manifestdocs tree
Cookbook (fd-59db20, draft)  cookbook.md
  Recipes (fd-1a2b3c, approved)  recipes/recipes.md
    Recipe search (fd-7c2e41, draft)  recipes/search/search.md

$ manifestdocs show fd-7c2e
Recipe search (fd-7c2e41, draft)
File: recipes/search/search.md
Parent: Recipes (fd-1a2b3c)
Summary: Finds recipes by ingredient, cuisine and cooking time.
...
Supporting docs:
- recipes/search/ranking-rules.md

$ manifestdocs check
recipes/search/search.md:3: error: `title` must be double-quoted
recipes/search/search.md:12: error: broken link `ranking.md`

3 nodes: 2 errors
```

| Command | What it does |
|---|---|
| `init [title]` | Starts a tree in the current folder |
| `tree [path] [--depth n] [--json]` | Outlines the nodes, from the root or from a node or folder |
| `context <id or path> [--budget n] [--out path]` | Prints a node's context for an agent, or writes it to a file |
| `show <id or path> [--json]` | Prints one node with its parent and supporting docs. An ID prefix of 4 to 6 hex characters works too |
| `check [path] [--json] [--offline]` | Validates frontmatter, IDs, links, anchors and node sizes, in the tree that contains `path` or the current folder. With a `github` block, it also checks linked issues through `gh`, unless `--offline` |
| `decide --file <path or -> [--dry-run] [--unattended] [--out dir] [--json]` | Writes a decision record and prints its path; see [Record a decision](#record-a-decision) |
| `decisions [id or path] [--json] [--ref ref]` | Lists decision records for a node and its ancestors, or for the whole tree |
| `work <id or path> [--json] [--ref ref]` | Lists the issues a node links to in `work`, with their sub-issues at any depth, each with its issue type, open or closed state and how many sub-issues are done. Needs `gh` and a `github` block, and exits 1 when an issue can't be read |
| `stale [--json]` | Lists the open linked issues whose node changed after their `spec-ref`, or after the docs commit when they were created, with a summary of the changed files. Needs `gh`, a `github` block and the docs clone's full history, and exits 1 when an issue is stale or can't be checked |

With `--json`, a command prints one JSON object for scripts, in the versioned schema in [the CLI spec](design/cli/cli.md#json-output), and exits with the same code as text. Paths in arguments and output are relative to the current folder. `check` exits 1 when the tree has errors, so it can gate docs PRs in CI, or on warnings too with `--strict`, and `show` exits 1 when the node's frontmatter has errors. Every command exits 2 when it can't run.

## Record a decision

Write the record without `id`, `date` or `status`, and pass it to `decide` with `--file`, or `--file -` for stdin:

```markdown
---
node: fd-7c2e41
title: "Search results show 20 recipes per page"
topic: search-page-size
decided_by: octocat
---

**Decision.** Search shows 20 recipes per page.

**Why.** Readers rarely scroll past 20 results before they refine a search.
```

```console
$ mfd decide --file record.md
recipes/search/decisions/2026-10-28-search-page-size.md
```

`decide` fills in the ID, today's date and the status: `accepted` under `decisions.merge: review`, the default, and `proposed` under `merge: proposed` or with `--unattended`. It writes only the record, plus `status: superseded` in the record it supersedes, and warns when the docs clone has uncommitted changes or commits that aren't on its default branch. It reports every problem in the input with its line and a fix, and writes nothing until all are fixed. Running it again with the same node, topic and body prints the existing path; a different body on the same topic needs `supersedes`.

`--dry-run` prints the record and its path without writing. `--unattended` writes to `.manifestdocs/decisions/` in the current repo, which ignores itself, for sandboxed runs whose docs mount is read-only. `--out <dir>` writes to another folder.

`mfd decisions fd-7c2e41` lists the records for a node and its ancestors, one line each.

## Use a tree from a code repo

Commands find the tree by walking up from the current folder to the repo root. When that finds none, they use the nearest `.manifestdocs/config.yaml` at or above the current folder, whose `tree` is relative to the folder holding `.manifestdocs/`.

In a single repo whose tree is in `docs/`, commit this at the repo root:

```yaml
# .manifestdocs/config.yaml
tree: docs
```

In a workspace folder that holds the code repos and a docs repo, put one beside them:

```yaml
# ~/work/cookbook/.manifestdocs/config.yaml
tree: cookbook-docs
```

`check`, `tree`, `show` and `decide` read the docs clone's working tree, so you see your own edits. From another repo, `context`, `decisions`, `work` and `stale` read the docs clone's default branch, `origin/HEAD`, so a clone left on a feature branch doesn't change what an agent reads. They never fetch, so they read the clone as of its last `git fetch`, and when `origin/HEAD` isn't set, they print the `git remote set-head` command that sets it. `tree`, `show`, `context`, `check`, `decisions` and `work` take `--ref <ref>` to read another branch, tag or commit.

## Develop

```sh
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

The spec is the tree in [design/](design/manifest-docs.md), which passes its own `check`.
