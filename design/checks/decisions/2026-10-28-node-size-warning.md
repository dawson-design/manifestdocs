---
id: dc-c86c70c
node: fd-4cb949
title: "Warn about nodes over 2,500 estimated tokens"
status: superseded
topic: node-size-warning
decided_by: alastair
date: 2026-10-28
---

**Decision.** From M1, `check` warns about each node whose body is over `node_tokens` estimated tokens, 2,500 by default, set in `manifestdocs.yaml`. It uses the context budget's estimate of characters divided by three.

**Why.** `context` includes a node and all its ancestors in full, which stays focused only while nodes stay small. Research on repo context files found that agents do worse and cost more when given requirements the task doesn't need. A tree can hold product and technical specs together, so the check limits size rather than content. Sharing the budget's unit lets authors compare the two. The largest nodes in the Manifest Docs design tree are about 2,200 tokens.

**Rejected.** A limit in words, a unit `context` doesn't use. Flagging code, file paths or identifiers in node bodies, which technical specs need. An error rather than a warning, since a large node still works.
