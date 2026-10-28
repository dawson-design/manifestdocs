---
id: dc-35e7a7d
node: fd-ae6745
title: "Decisions get their own PR in a single repo too"
status: accepted
topic: single-repo-pr
decided_by: alastair
date: 2026-10-28
---

**Decision.** When the tree is in the working repo, the `manifestdocs-decide` skill still creates a worktree on a new branch from the default branch and opens a PR for the record alone.

**Why.** Decisions are reviewed on their own, whatever repo holds the tree, and one write path serves both layouts.

**Rejected.** Writing the record into the current branch, so it merges with the code it explains.
