---
id: dc-423a914
node: fd-e54a4d
title: "The root node is optional"
status: accepted
topic: root-node-optional
decided_by: alastair
date: 2026-10-28
---

**Decision.** Without a root node, top-level nodes have no parent, `context` leaves out the project section, and `check` warns.

**Why.** Cargo workspaces work without a root package, and Hugo builds a home page with no content. The `check` warning separates a tree with no project rules from a forgotten file.

**Rejected.** Requiring a root node, as Antora requires a start page.
