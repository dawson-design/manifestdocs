---
id: fd-e54a4d
title: "Tree format"
status: draft
summary: "How a docs repo marks a tree, its nodes, the files each node owns, and their IDs."
---

# Tree format

## Root marker

A `manifestdocs.yaml` file marks the root of a tree. It can go in any folder of a docs repo, and one repo can contain several trees. A `manifestdocs.yaml` inside an existing tree starts a separate tree, and the outer tree skips that folder. The tool locates the root by walking up from the current directory to the repo root, then through the nearest [pointer](#pointer).

```yaml
# manifestdocs.yaml
name: cookbook
github:
  host: github.com
  docs_repo: example-org/cookbook-docs
decisions:
  merge: review          # review | proposed
statuses: [draft, approved]
node_tokens: 2500        # from M1
```

Only `name` is required. `merge` defaults to `review`, `statuses` defaults to `[draft, approved]`, and `node_tokens`, the node size over which `check` warns, defaults to 2,500. `superseded` is always a valid status, because the tool needs to know which status requires a replacement node, so `statuses` lists only the others. Without a `github` block, the tree commands still work and the work commands are off.

## Pointer

Walking up never reaches the root when the tree is in a subfolder of the code repo, such as `docs/`, or in a docs repo cloned beside the code repos. A pointer file, `.manifestdocs/config.yaml`, names the tree for every folder under the folder it's in. `tree` is relative to that folder.

```yaml
# cookbook/.manifestdocs/config.yaml: a single repo, committed
tree: docs
```

```yaml
# ~/work/cookbook/.manifestdocs/config.yaml: a workspace folder holding the code repos and the docs repo
tree: cookbook-docs
```

The nearest pointer at or above the current folder wins, so a repo's own pointer comes before one in a folder above it. A `.manifestdocs/` folder without `config.yaml`, such as one holding only decision drafts, isn't a pointer. Whether the tree is in the working repo follows from where the pointer leads, and isn't a setting.

## Which version the tool reads

Inside the docs clone, the tool reads the working tree, so authors see their edits and `check` validates the checked-out branch. When the tree is in the working repo, the working repo is the docs clone, so the tool always reads its working tree. The context header then shows the branch and any uncommitted changes, and warns when the clone isn't on its default branch. From any other repo, the tool reads the docs repo's default branch, which it resolves from `origin/HEAD`, so a docs clone left on a feature branch doesn't change an agent's context. When `origin/HEAD` is unset, the tool fails and prints the command that sets it, `git remote set-head origin --auto`, rather than guessing a branch name. The tool never fetches, so the header shows the commit and its date, and the ref is as current as the clone's last fetch. A `--ref` flag overrides both.

## Nodes

A folder under the root is a feature node when it contains a Markdown file named after the folder, such as `recipes/recipes.md`, with an `id` in its frontmatter. Because each node file has its own name, editor tabs and search results show which feature a file belongs to. The root node is named after the tree's `name` in `manifestdocs.yaml`, such as `cookbook.md`, because a clone's folder name isn't fixed. The root node is optional: without one, top-level nodes have no parent, `context` leaves out the project section, and `check` warns. A node's parent is the nearest ancestor folder that is a node. Folders in between are passed through, so a tree can follow a repo's existing folder layout. Node folder names default to kebab case, such as `key-strategies/`, so relative links need no escaped spaces. `init` and the `manifestdocs-author` skill create folders and node files that way, and `check` accepts existing names. When a folder is renamed without its node file, `check` reports the mismatch, because the node would otherwise drop out of the tree. A `decisions/` subfolder contains the node's [decision records](../decision-records/decision-records.md), so a folder named `decisions` is never a node.

## Ownership

A node owns every file under its folder, except files under a deeper node's folder or a nested tree. Those files are its supporting docs: `show` lists them, and `stale` watches them. `context` loads only the ones the node links to (see [agent context](../context/context.md)).

For example, `recipes/search/search.md`:

```markdown
---
id: fd-7c2e41
title: "Recipe search"
status: approved
summary: "Finds recipes by ingredient, cuisine and cooking time."
repos: [example-org/cookbook-web]
work: [example-org/cookbook-docs#41]
superseded_by: null
---

Requirements, acceptance criteria and open decisions for the node, with links
to supporting docs such as the [ranking rules](ranking-rules.md) and the
[search ADR](../../adr/2026-09-search-engine.md).
```

| Field | Required | Rule |
|---|---|---|
| `id` | Yes | `fd-` plus 6 random hex characters, unique within the tree |
| `title` | Yes | Plain name, double-quoted |
| `status` | Yes | One of the statuses in `manifestdocs.yaml`, or `superseded` |
| `summary` | Yes | One sentence, double-quoted, used in child and linked-node summaries |
| `repos` | No | Code repos that implement the node |
| `work` | No | Top-level work items for the node. Their sub-issues link through the hierarchy |
| `superseded_by` | When superseded | ID of the replacing node |

`title` and `summary` are always double-quoted. An unquoted `: ` breaks the YAML, an unquoted ` #` silently cuts the value short, and a bare `true` or `null` isn't text. `check` warns about unknown fields, so teams can add their own while typos still show. How to write the body, including requirements and scenarios, is in [node body](node-body/node-body.md).

## Links

Images, diagrams, style guides and external references, such as a design file or a platform's guidelines, are ordinary Markdown links in the node body. Mermaid diagrams go inline. Links between nodes use relative paths, so they work on GitHub. `check` confirms that local links resolve. The tool doesn't fetch or track external links.

## Stable IDs

References use node IDs, as in `superseded_by`, a decision record's `node`, and an issue that names its node. They stay valid when a folder moves, unlike body links, which are relative paths that `check` reports when they break. Output shows each node's current path. IDs are a prefix plus lowercase hex from the operating system's random source: `fd-` and 6 characters for nodes, `dc-` and 7 for decision records. The tool generates a new ID when one matches an existing ID in the tree, and `check` reports duplicates, such as two branches that picked the same ID. Stored IDs always have their full length. Commands accept a prefix of 4 or more hex characters, and fail with a list of the matches when a prefix matches more than one ID. Records are superseded rather than deleted, so their IDs stay in the tree.
