---
id: dc-17e9f87
node: fd-e54a4d
title: "A nested manifestdocs.yaml starts its own tree"
status: accepted
topic: nested-trees
decided_by: alastair
date: 2026-10-28
---

**Decision.** A `manifestdocs.yaml` inside an existing tree starts a separate tree, and the outer tree skips that folder. `check` lists each tree it skipped.

**Why.** Go modules, git repositories and Bazel workspaces all let the nearest marker govern the files under it, with no exclude list.

**Rejected.** Merging nested trees into the outer one, as EditorConfig cascades settings. That conflicts with IDs being unique within a tree, and ESLint has moved away from cascading config.
