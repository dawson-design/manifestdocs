---
id: dc-2eed984
node: fd-ae6745
title: "decide writes accepted under review, proposed otherwise"
status: accepted
topic: status-on-write
decided_by: alastair
date: 2026-10-28
---

**Decision.** Under `merge: review`, `decide` writes `accepted`, because the PR review is the approval. Under `merge: proposed` and in unattended runs, it writes `proposed`. Reviewers reject a record by merging it as `rejected`, not by closing the PR. A record gets the same status whoever drafted it.

**Why.** In the Rust RFC process and the UK GDS guidance, merging means accepted. A merged rejected record stops the same idea from coming back, as AWS's ADR guidance recommends. Status says whether a decision is in force; `decided_by` and git history say who made it.

**Rejected.** Accepting in a separate step after merge, as Python PEPs do, which someone has to remember to do.
