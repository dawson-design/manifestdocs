---
id: dc-840d43d
node: fd-ae6745
title: "decide writes only the file"
status: superseded
topic: decide-writes-file-only
decided_by: alastair
date: 2026-10-28
---

**Decision.** `decide` writes the record and prints its path. It never branches or commits. The `manifestdocs-decide` skill creates a branch in a git worktree of the docs repo, then commits, pushes and opens the PR.

**Why.** Changesets, towncrier, scriv and reno all write a file and leave git to the user. A worktree keeps the main docs clone on its default branch, so other sessions' context doesn't change.

**Rejected.** Writing to a `decisions/<user>` branch in the docs clone, which switches its branch under editors and other sessions. Writing commits with git plumbing, which rebuilds git inside the tool.
