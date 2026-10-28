---
id: dc-5379928
node: fd-622eed
title: "GitHub is optional"
status: accepted
topic: github-optional
decided_by: alastair
date: 2026-10-28
---

**Decision.** Tree commands work with only `manifestdocs.yaml`. Work commands turn on with a `github` block.

**Why.** The tree, agent context and decision records are useful offline and in sandboxes with no token.
