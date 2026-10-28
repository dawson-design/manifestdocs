---
id: dc-b9b15a1
node: fd-ae6745
title: "Decisions cost one line each in context"
status: accepted
topic: decision-size
decided_by: alastair
date: 2026-10-28
---

**Decision.** `context` shows each decision as one line: its ID, status and title, and the first sentence of its **Why.** paragraph. A superseding record's line ends with the older record's ID and title, read from the older file. Each node's decisions follow it in the fill order, newest first. `check` warns when a record's body is over 300 estimated tokens. When a decision changes behaviour, the same docs PR edits the node's requirements, and a superseding record names the old choice under **Rejected**.

**Why.** Records only accumulate, and `context` includes the decisions of every ancestor. Without these rules they would outgrow the nodes they belong to. One line per record caps the cost whatever a record's length or count. Reading the predecessor's title from its file keeps each fact in one place, unlike a summary copied into the new record. Moving behaviour into the spec leaves records with only the why.

**Rejected.** Copying a one-line summary of the superseded record into the new one, which duplicates text and grows with each supersession. A limit on records per node, which one line each makes unnecessary. A configurable record limit, which would add a key to `manifestdocs.yaml` for a size the record format already fixes.
