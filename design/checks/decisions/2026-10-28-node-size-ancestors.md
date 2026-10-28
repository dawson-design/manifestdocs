---
id: dc-5cdcaff
node: fd-4cb949
title: "The node size warning applies only to nodes with child nodes"
status: accepted
topic: node-size-warning
decided_by: alastair
date: 2026-10-28
supersedes: dc-c86c70c
---

**Decision.** `check` warns about a node over `node_tokens` only when the node has child nodes. The warning suggests moving detail into child nodes or a linked supporting doc.

**Why.** The limit exists because `context` loads every ancestor in full for each node below it. A node with no child nodes is loaded in full only when it's the target, so its length costs only the agents working on that feature, who need it all. Complex features can then be written in full, and shared ancestors stay small.

**Rejected.** Warning about every node, as dc-c86c70c did, which pushed complex leaf features to split into child nodes and so made them ancestors, the kind whose size every descendant pays for.
