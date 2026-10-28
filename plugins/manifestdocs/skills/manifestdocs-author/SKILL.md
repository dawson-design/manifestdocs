---
name: manifestdocs-author
description: Writes or edits a Manifest Docs feature node so it passes manifestdocs check, with a unique ID, double-quoted title and summary, and relative links. Use when adding a feature to a Manifest Docs tree, proposing a change to a feature's requirements, drafting a node from existing code, editing a node's frontmatter, moving or retiring a node, or fixing problems manifestdocs check reports.
allowed-tools: Bash(manifestdocs check:*), Bash(manifestdocs tree:*), Bash(manifestdocs show:*), Bash(openssl rand -hex 3)
---

# Write a valid node

A tree is the folder with `manifestdocs.yaml`. Each feature is a folder with a node file named after the folder, such as `recipes/search/search.md`. The root node is `<name>.md` in the tree's folder, where `<name>` comes from `manifestdocs.yaml`.

## Add a node

1. Run `manifestdocs tree` to see where the feature belongs. A node's parent is the nearest folder above it that is a node.
2. Make the folder in kebab case, such as `recipe-import/`, and the node file inside it with the same name: `recipe-import/recipe-import.md`. Never name a folder `decisions`, which holds decision records.
3. Make the ID with `openssl rand -hex 3` and put `fd-` in front. Run `manifestdocs show fd-<hex>`. It must say `no node`; if not, make another.
4. Write the frontmatter:

   ```markdown
   ---
   id: fd-1a2b3c
   title: "Recipe import"
   status: draft
   summary: "Imports recipes from a URL or a file."
   repos: [example-org/cookbook-web]
   work: [example-org/cookbook-docs#58]
   ---
   ```

   - `title` and `summary` are always double-quoted. Escape a `"` inside them as `\"`. The summary is one sentence.
   - `status` is one of the `statuses` in `manifestdocs.yaml`, which default to `draft` and `approved`. Use `draft` for a new node.
   - `repos` and `work` are optional lists, with `owner/name` for repos and `owner/name#number` for issues.
5. Write the body: what the feature must do. That's its behaviour, its contracts, the constraints that apply to every feature below it, and open questions. Write each testable requirement as a heading with at least one scenario:

   ```markdown
   ## Requirements

   ### Requirement: Filter by cooking time
   Search SHALL let readers limit results to recipes they can cook within a chosen time.

   #### Scenario: Under 30 minutes
   - WHEN a reader picks "under 30 minutes"
   - THEN every result has a total time of 30 minutes or less
   ```

   One behaviour per requirement, stated with SHALL or MUST, and scenarios a test could check. If the implementation could change without changing visible behaviour, the detail doesn't go in the body. Put it in a supporting doc in the node's folder, such as `design.md`, and link it, or in a child node when it's a feature of its own. Put supporting docs, images and diagrams in the node's folder and link them with relative paths. Mermaid diagrams go inline. External references are plain links.

## Propose a change

When the user wants to change what a feature does, change the spec before the code:

1. Run `manifestdocs tree` and find the nodes the change touches. Say which ones you picked.
2. Add, change or remove requirements and scenarios in those nodes. Behaviour that no node covers gets a new node (see "Add a node").
3. Run `manifestdocs check`, then show the user the changed requirements for review. Commit or open a pull request only when the user asks.

## Draft a node from existing code

For a feature that's already built but has no node:

1. Read the feature's code, tests and any existing docs.
2. Write the behaviour you can observe as requirements, and take scenarios from existing tests where they fit.
3. List what the code doesn't settle under `## Open questions` instead of guessing.
4. Use `status: draft`, and split a large area into child nodes, one per feature.

## Edit, move or retire a node

- Never change an existing node's `id`, because references to it use the ID.
- When you rename a node's folder, rename its node file to match, and fix the links that `check` then reports.
- To retire a node that another node replaces, keep its file, set `status: superseded`, and add `superseded_by:` with the replacement node's ID.
- To remove a feature with no replacement, first move any child node that stays, then delete the node's folder. Ask the user why, and write it as a decision record on the parent node. Then fix the links and references that `check` reports.

## Check

Run `manifestdocs check` after every change to a tree. It prints `path:line: severity: message` lines. Fix every error in the files you touched, and run it again until it exits 0. Warnings, such as an unknown field, are allowed but worth a second look. A node with child nodes that's over the size limit is loaded in full for every feature below it: move detail into a child node or a linked supporting doc. A node with no child nodes has no limit.
