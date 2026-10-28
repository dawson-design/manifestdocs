# Manifest Docs

Manifest Docs turns a folder of Markdown in a docs repo into a feature tree that people and coding agents can walk. Each feature is a folder with one node file for its requirements and decisions, and every file remains an ordinary Markdown page on GitHub, readable without the tool. The `manifestdocs` binary reads the tree from disk. It has no server or account, and it never writes to GitHub.

## Install

Build from a clone of this repo, with Rust installed:

```sh
cargo install --path crates/manifestdocs
```

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
| `tree [path] [--depth n]` | Outlines the nodes, from the root or from a node or folder |
| `context <id or path> [--budget n]` | Prints a node's context for an agent |
| `show <id or path>` | Prints one node with its parent and supporting docs. An ID prefix of 4 to 6 hex characters works too |
| `check [path]` | Validates frontmatter, IDs, links, anchors and node sizes, in the tree that contains `path` or the current folder |

Paths in arguments and output are relative to the current folder. `check` exits 1 when the tree has errors, so it can gate docs PRs in CI, or on warnings too with `--strict`, and `show` exits 1 when the node's frontmatter has errors. Every command exits 2 when it can't run.

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

## Develop

```sh
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

The spec is the tree in [design/](design/manifest-docs.md), which passes its own `check`.
