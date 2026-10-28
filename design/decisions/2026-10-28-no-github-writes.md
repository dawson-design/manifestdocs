---
id: dc-27171ba
node: fd-622eed
title: "The binary never writes to GitHub"
status: accepted
topic: no-github-writes
decided_by: alastair
date: 2026-10-28
---

**Decision.** `mfd` reads issues through `gh` and never writes. Agents create issues, comments and pull requests with `gh`, guided by the plugin's skills.

**Why.** When a CLI writes to GitHub internally, a harness's permission rules see only the CLI command, so its writes skip per-action approval. With every write in a visible `gh` call, a team can allow all `mfd` commands without a prompt.

**Rejected.** `mfd plan` creating issues, `mfd capture` commenting on issues, and `stale` posting comments on a schedule.
