---
id: fd-4cb949
title: "Checks"
status: draft
summary: "Validates a tree and its decision records, so `check` can gate docs PRs in CI."
---

# Checks

`mfd check` exits non-zero on errors so it can gate CI on the docs repo. With `--strict`, warnings fail it too, for a team that wants CI to hold the tree to them, such as the node size limit. The gate blocks merges only when the docs repo's branch rules require the check.

- Every node has valid frontmatter, a unique ID, and a double-quoted `title` and `summary`.
- Every reference and link resolves: node IDs, `superseded_by`, and local Markdown links and their anchors in node bodies and supporting docs.
- Superseded nodes name a replacement that exists and isn't the node itself.
- A Markdown file with a node `id` that isn't a node file, such as after a folder or tree rename, is an error.
- Every decision record has valid frontmatter, a node that exists, and a `supersedes` reference that resolves. An unknown field in a record's frontmatter is a warning, as in a node's.
- At most one accepted decision per `topic` within a node.
- From M2, a warning for each decision record whose body is over 300 estimated tokens, a fixed limit, since a record is [a few lines](../decision-records/decision-records.md#record-format) in any tree.
- With GitHub access, every referenced issue exists, and issues that name a node point to one that exists.
- Warnings: a tree with no root node, one line for each nested tree it skipped, unknown fields in a node's frontmatter, and a file in a node file's position with frontmatter but no `id`.
- From M1, a warning for each node with child nodes whose body is over `node_tokens` in `manifestdocs.yaml`, 2,500 by default, estimated as the [context budget](../context/context.md) estimates tokens. `context` includes the target node and its ancestors in full, so a large ancestor grows the context of every node below it. A node with no child nodes is loaded in full only as the target, so a complex feature there can be as long as it needs. The warning gives the node's estimated size and suggests moving detail into child nodes or a linked supporting doc.
- Optional: issue bodies that copy long passages of spec text instead of linking to the node.
