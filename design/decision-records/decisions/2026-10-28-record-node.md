---
id: dc-be75397
node: fd-ae6745
title: "A decision for several nodes goes on their nearest common ancestor"
status: accepted
topic: record-node
decided_by: alastair
date: 2026-10-28
---

**Decision.** A decision that applies to several nodes is recorded on their nearest common ancestor. One that applies to no node is recorded on the root node.

**Why.** `context` loads every ancestor's decisions, so a record on the common ancestor reaches each node it applies to, and on no others above it. The record format stays one node per record.

**Rejected.** Asking the engineer every time, which leaves the same choice to each session. A list of nodes in the record's `node` field, which changes the record format, `check` and `context`.
