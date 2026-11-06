---
id: dc-3aef410
node: fd-ae6745
title: "decide writes accepted under review and direct, proposed otherwise"
status: accepted
topic: status-on-write
decided_by: alastair
date: 2026-10-29
supersedes: dc-2eed984
---

**Decision.** Under `changes: review` and `changes: direct`, `decide` writes `accepted`. Under `changes: proposed` and in unattended runs, it writes `proposed`. Reviewers reject a record by merging it as `rejected`, not by closing the PR. A record gets the same status whoever drafted it.

**Why.** Status says whether a decision is in force: under `review` the PR review is the approval, and under `direct` the team has chosen to put changes in force without one.

**Rejected.** Writing `proposed` under `direct` and accepting with a later push, which leaves every record waiting on a step the team chose to skip. Keying status on `decisions.merge`, which `changes` replaces.
