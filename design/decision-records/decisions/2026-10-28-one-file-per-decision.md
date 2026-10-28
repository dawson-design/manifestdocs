---
id: dc-a7f5023
node: fd-ae6745
title: "One file per decision in the node's decisions folder"
status: accepted
topic: one-file-per-decision
decided_by: alastair
date: 2026-10-28
---

**Decision.** Each decision is a Markdown file in its node's `decisions/` folder, named by date and topic, written by `mfd decide` and shared through the docs repo.

**Why.** Separate files avoid merge conflicts when several sessions write at once, and the docs repo gives every engineer and agent the same records.

**Rejected.** The harness's local memory, which only one user sees.
