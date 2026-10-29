---
id: dc-b9b15a1
node: fd-ae6745
title: "Decisions cost one line each in context"
status: accepted
topic: decision-size
decided_by: alastair
date: 2026-10-28
---

**Decision.** `context` shows each decision as one line, ending with the ID and title of any record it supersedes. `check` warns about a record body over 300 estimated tokens. A decision that changes behaviour also edits the node's requirements, in the same docs PR.

**Why.** Records only accumulate, and `context` includes every ancestor's decisions, so they would otherwise outgrow their nodes. One line each caps the cost whatever a record's length or count. Reading the older title from its file keeps each fact in one place. Moving behaviour into the spec leaves records with only the why.

**Rejected.** Copying a summary of the superseded record into the new one, which duplicates text and grows with each supersession. A limit on records per node, which one line each makes unnecessary. A configurable record limit, a `manifestdocs.yaml` key for a size the record format already fixes.
