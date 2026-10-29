---
id: dc-e8cadfd
node: fd-e54a4d
title: "Other repos read the docs repo's default branch"
status: superseded
topic: read-version
decided_by: alastair
date: 2026-10-28
---

**Decision.** Inside the docs clone, the tool reads the working tree and warns when the clone is off its default branch. From any other repo, it reads the default branch from `origin/HEAD`, never fetches, and shows the commit and its date in the context header. It fails rather than guessing when `origin/HEAD` is unset.

**Why.** A docs clone left on a feature branch would otherwise change every agent's context. Reading the merged branch also keeps unreviewed records, already marked `accepted`, out of other sessions.

**Rejected.** Reading only the working tree until M3.
