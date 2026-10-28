---
id: fd-e54a4d
title: Tree format
status: draft
summary: How a docs repo marks a tree, its nodes, the files each node owns, and their IDs.
---

# Tree format

## Root marker

A `manifestdocs.yaml` file marks the root of a tree. It can go in any folder of a docs repo, and one repo can contain several trees. A `manifestdocs.yaml` inside an existing tree starts a separate tree, and the outer tree skips that folder. The tool locates the root by walking up from the current directory, then through the user configuration below.

```yaml
# manifestdocs.yaml
name: event-bus
github:
  host: github.com
  docs_repo: example-org/eeb-docs
decisions:
  merge: review          # review | proposed
statuses: [draft, approved, superseded]
```

Only `name` is required. Without a `github` block, the tree commands still work and the work commands are off.

## User configuration

Engineers usually work in a code repo cloned next to the docs repo, and walking up from there never reaches the root. A user-level file maps workspace folders to tree roots.

```yaml
# ~/.config/manifestdocs/config.yaml
workspaces:
  - path: ~/work/eeb             # sessions anywhere under this folder
    tree: ~/work/eeb/eeb-docs    # the folder with manifestdocs.yaml
```

## Which version the tool reads

Inside the docs clone, the tool reads the working tree, so authors see their edits and `check` validates the checked-out branch. The context header then shows the branch and any uncommitted changes, and warns when the clone isn't on its default branch. From any other repo, the tool reads the docs repo's default branch, which it resolves from `origin/HEAD`, so a docs clone left on a feature branch doesn't change an agent's context. When `origin/HEAD` is unset, the tool fails and prints the command that sets it, `git remote set-head origin --auto`, rather than guessing a branch name. The tool never fetches, so the header shows the commit and its date, and the ref is as current as the clone's last fetch. A `--ref` flag overrides both.

## Nodes

A folder under the root is a feature node when it contains a Markdown file named after the folder, such as `recipes/recipes.md`, with an `id` in its frontmatter. Because each node file has its own name, editor tabs and search results show which feature a file belongs to. The root node is named after the tree's `name` in `manifestdocs.yaml`, such as `event-bus.md`, because a clone's folder name isn't fixed. The root node is optional: without one, top-level nodes have no parent, `context` leaves out the project section, and `check` warns. A node's parent is the nearest ancestor folder that is a node. Folders in between are passed through, so a tree can follow a repo's existing folder layout. Node folder names default to kebab case, such as `key-strategies/`, so relative links need no escaped spaces. `init` and the `manifestdocs-author` skill create folders and node files that way, and `check` accepts existing names. When a folder is renamed without its node file, `check` reports the mismatch, because the node would otherwise drop out of the tree. A `decisions/` subfolder contains the node's [decision records](../decision-records/decision-records.md).

## Ownership

A node owns every file under its folder, except files under a deeper node's folder or a nested tree. Those files are its supporting docs: `show` lists them, and `stale` watches them. `context` loads only the ones the node links to (see [agent context](../context/context.md)).

For example, `platform/dedup/dedup.md`:

```markdown
---
id: fd-7c2e41
title: Universal De-Duplicator
status: approved
summary: Drops repeat events across producers using a shared key and a time window.
repos: [example-org/eeb-dedup]
work: [example-org/eeb-docs#41]
superseded_by: null
---

Requirements, acceptance criteria and open decisions for the node, with links
to supporting docs such as the [key rules](dedup-keys.md) and the
[window ADR](../../adr/2026-09-dedup-window.md).
```

| Field | Required | Rule |
|---|---|---|
| `id` | Yes | `fd-` plus 6 random hex characters, unique within the tree |
| `title` | Yes | Plain name |
| `status` | Yes | One of the statuses in `manifestdocs.yaml` |
| `summary` | Yes | One sentence, used in ancestor and child summaries |
| `repos` | No | Code repos that implement the node |
| `work` | No | Top-level work items for the node. Their sub-issues link through the hierarchy |
| `superseded_by` | When superseded | ID of the replacing node |

## Links

Images, diagrams, style guides and external references, such as a design file or a platform's guidelines, are ordinary Markdown links in the node body. Mermaid diagrams go inline. Links between nodes use relative paths, so they work on GitHub. `check` confirms that local links resolve. The tool doesn't fetch or track external links.

## Stable IDs

Links use node IDs. Output shows paths for readability. When a folder moves, links stay valid and `check` reports the new path. IDs are a prefix plus lowercase hex from the operating system's random source: `fd-` and 6 characters for nodes, `dc-` and 7 for decision records. The tool generates a new ID when one matches an existing ID in the tree, and `check` reports duplicates, such as two branches that picked the same ID. When reading, the tool accepts IDs of 4 or more characters. Records are superseded rather than deleted, so their IDs stay in the tree.
