---
id: fd-0e0a4e
title: "Links to work"
status: draft
summary: "Links nodes to GitHub issues, reads their sub-issue hierarchy, and reports issues whose requirements changed after planning."
---

# Links to work

## Links

- A feature node lists its top-level work items in `work`. Their sub-issues link to the node through the hierarchy, so their bodies need no link.
- When a deeper node lists a sub-issue in its own `work`, the deeper node applies to that issue and its sub-issues.
- An issue outside that hierarchy names its node with a line in its body: `feature: fd-7c2e41`.
- An issue can record the docs commit it was planned against: `spec-ref: <sha>`. Without one, the tool uses the last commit on the docs repo's default branch before the issue was created, so existing issues work without edits.
- To find an issue's node, `context` walks up from the issue through its parents, and the first issue that a node lists in `work` or that has a `feature:` line decides. On one issue, `work` comes first, even from a node with frontmatter errors, and of two `feature:` lines, the first. A parent `gh` can't read ends the walk. An issue that links to no node, or none below a parent `gh` can't read, or whose deciding `feature:` line is malformed or names a node not in the tree, stops `context` with an error.
- The tool reads both directions and never writes a second copy of a link.
- Manifest Docs doesn't use labels or Projects fields for node links. A label per node clutters a repo's label list. Projects fields are stored outside the issue, need a project-scoped token, and differ from project to project.

## Work hierarchy

Manifest Docs reads GitHub's sub-issues, at any depth. In a repo without them, every issue stands alone. Level names such as initiative, epic or task are display labels only: the tool reads them from issue types when present and never requires them. `context` and `work` show the chain of parents and children that sub-issues return.

## Staleness

`mfd stale` compares each open linked issue's `spec-ref`, or its creation-time commit, with the latest commit that touched any file its node owns (see [ownership](../tree-format/tree-format.md#ownership)), images included. For each stale issue it prints the issue, the node and a diff summary of what changed. It only reports. A person or an agent decides whether to comment on the issue.

A node's linked issues are its work items, their sub-issues unless a deeper node lists them, and the issues whose `feature:` line names it, with their sub-issues. `stale` reads the docs version `work` reads and compares commits up to it, so uncommitted edits don't count. Folders that were deeper nodes, nested trees or records folders at the baseline are left out too, so a child node that moved isn't the parent's change; there, as now, a node is a node file whose `id` parses. A change that touches only the node file, present at both commits, and in it only the `work` list, doesn't count, since planning adds an issue to `work` after setting its `spec-ref`.

An issue is listed as not checked when `gh` can't read it, when its `spec-ref` isn't a commit in the history read, such as one on a branch that was squash-merged or never merged, or when the clone can't read the node's history or files, as a partial clone without them can't, since the tool never fetches. A shallow clone is an error, because its history stops early: fetch all of it first.

## Writing to GitHub

The binary never writes to GitHub. Agents create issues, comments and pull requests with `gh`, guided by the plugin's skills. The harness's permission rules see each write, and a team can allow every `mfd` command without a prompt.

- **Planning work.** With the `manifestdocs-plan-issue` skill, the agent reads a node and drafts work items with acceptance criteria, using the repo's own issue templates. After the user approves, the agent creates the issues with `gh`, sets parents as sub-issues, and adds the top-level items to the node's `work` field.
- **Capturing a decision from an issue.** With the `manifestdocs-capture` skill, the agent reads the thread with `gh issue view`, drafts a decision record, ADR or spec change, and writes a record with `mfd decide`. A person opens the pull request.
