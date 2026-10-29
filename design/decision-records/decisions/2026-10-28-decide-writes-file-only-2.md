---
id: dc-e6640dd
node: fd-ae6745
title: "decide writes the record and marks the one it supersedes"
status: accepted
topic: decide-writes-file-only
decided_by: "alastair"
date: 2026-10-28
supersedes: dc-840d43d
---

**Decision.** `decide` writes the record and, when it supersedes another, changes only that record's `status` line to `superseded`. It never branches or commits; the `manifestdocs-decide` skill does that in a worktree.

**Why.** Under `merge: review` the new record is `accepted`, so leaving the old one `accepted` made `check` fail on two accepted records with one topic. Both edits belong in the same docs PR.

**Rejected.** Warning and leaving the edit to the skill, which every caller would have to repeat. Letting `check` treat a superseded-but-accepted record as superseded, which hides a stale status.
