---
id: fd-4cb949
title: "Checks"
status: draft
summary: "Validates a tree and its decision records, so `check` can gate docs PRs in CI."
---

# Checks

`mfd check` exits non-zero on errors so it can gate CI on the docs repo. The gate blocks merges only when the docs repo's branch rules require the check.

- Every node has valid frontmatter, a unique ID, and a double-quoted `title` and `summary`.
- Every reference and link resolves: node IDs, `superseded_by`, and local Markdown links and their anchors in node bodies and supporting docs.
- Superseded nodes name a replacement that exists and isn't the node itself.
- A Markdown file with a node `id` that isn't a node file, such as after a folder or tree rename, is an error.
- Every decision record has valid frontmatter, a node that exists, and a `supersedes` reference that resolves.
- At most one accepted decision per `topic` within a node.
- With GitHub access, every referenced issue exists, and issues that name a node point to one that exists.
- Warnings: a tree with no root node, one line for each nested tree it skipped, unknown fields in a node's frontmatter, and a file in a node file's position with frontmatter but no `id`.
- Optional: issue bodies that copy long passages of spec text instead of linking to the node.
