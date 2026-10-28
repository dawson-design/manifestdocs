---
id: dc-1bea476
node: fd-fd80a6
title: "The plugin offers a five-step workflow"
status: accepted
topic: workflow-loop
decided_by: alastair
date: 2026-10-28
---

**Decision.** The plugin's commands follow one workflow: explore, propose, apply, verify and record. `/manifestdocs:explore` replaces `/manifestdocs:context`, and `/manifestdocs:record` replaces `/manifestdocs:decide`. A new `manifestdocs-verify` skill checks a change against its node's requirements, scenarios and accepted decisions, and only reports.

**Why.** Five named steps are easier to learn than a set of separate skills, and a verify step catches a change that drifts from its spec before review. Proposing spec changes as part of each change lets a tree grow from the work on an existing codebase. The steps are skills because each needs judgment, and the binary never calls a model.

**Rejected.** Change folders with delta specs that a later step merges into the tree, which repeat git branches and pull requests. A tasks file per change, which repeats GitHub Issues.
