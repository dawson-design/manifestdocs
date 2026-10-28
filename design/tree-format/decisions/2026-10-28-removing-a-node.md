---
id: dc-a504dff
node: fd-e54a4d
title: "A removed feature's node is deleted, with the reason recorded on its parent"
status: accepted
topic: removing-a-node
decided_by: alastair
date: 2026-10-28
---

**Decision.** When the software drops a feature with no replacement, its node's folder is deleted, along with its supporting docs, decision records and child nodes. A decision record on the parent node says why. `superseded` stays reserved for a node that another node replaces.

**Why.** Git history already keeps deleted files, and `check` already finds the links and IDs left dangling. The record on the parent reaches agents working on sibling features through `context`, so the feature doesn't come back without a new reason, as a rejected decision does.

**Rejected.** A `removed` or `retired` status that keeps the node in the tree, which `tree` and `context` would still show as a feature to build unless every command learned to skip it. Superseding the node with its parent, which claims a replacement that doesn't exist.
