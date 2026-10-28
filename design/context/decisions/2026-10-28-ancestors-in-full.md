---
id: dc-053c6a5
node: fd-54bf4e
title: "Context includes every ancestor in full"
status: accepted
topic: ancestors-in-full
decided_by: alastair
date: 2026-10-28
---

**Decision.** After the target node, `context` includes its ancestors in full, from the parent up to the root node, then linked docs, the issue and child summaries.

**Why.** A constraint on a mid-level node, such as "every search filter works offline", applies to each node below it, and a summary drops it. The root node already went in full for that reason. In a well-decomposed tree the ancestors stay small, and the node size warning in `check` keeps them that way. The budget fills in order, so the parent comes before the root, but an ancestor that doesn't fit is left out while a later, smaller one still goes in.

**Rejected.** The root in full with parent and ancestor summaries, which hid the constraints of the nodes in between. A marked constraints section that `context` extracts from each ancestor, which adds a format rule.
