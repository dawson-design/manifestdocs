---
id: dc-28355cd
node: fd-fd80a6
title: "Decisions are recorded in the session or on demand, with no end-of-session hook"
status: accepted
topic: no-session-end-hook
decided_by: "alastair"
date: 2026-10-28
---

**Decision.** The plugin has no end-of-session hook. The `manifestdocs-decide` skill triggers during the session when a decision is made, and `/manifestdocs:record` reviews the session on demand.

**Why.** Claude Code's `SessionEnd` hook can't continue a session, and `Stop` fires after every turn, so a once-per-session prompt fires after the first turn, before most decisions are made.

**Rejected.** Prompting at the first `Stop`, which comes too early to catch most decisions. Prompting after turns that edited files, with a rate limit, which still guesses when a session is ending and interrupts the engineer.
