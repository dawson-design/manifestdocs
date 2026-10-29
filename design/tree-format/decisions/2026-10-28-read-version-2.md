---
id: dc-fa95b03
node: fd-e54a4d
title: "Only agent-facing reads use the default branch from other repos"
status: accepted
topic: read-version
decided_by: "alastair"
date: 2026-10-28
supersedes: dc-e8cadfd
---

**Decision.** From another repo, `context`, `decisions` and `work` read the docs clone's `origin/HEAD`. `check`, `tree` and `show` read its working tree. `--ref` overrides either, and the tool never fetches.

**Why.** Agent context must not change because a clone sits on a feature branch, and unreviewed records must stay out of other sessions. But an author checking their own edits from a code repo needs `check`, `tree` and `show` to see them; reading `origin/HEAD` made `check` pass on edits it never read.

**Rejected.** Every read command reading `origin/HEAD`, with the author skill passing a flag to see edits, which every caller would have to remember.
