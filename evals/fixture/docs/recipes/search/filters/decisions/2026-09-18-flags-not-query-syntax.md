---
id: dc-b60f720
node: fd-3e8169
title: "Filters are flags, not query syntax"
status: accepted
topic: filter-syntax
decided_by: priya
date: 2026-09-18
---

**Decision.** Each filter is its own flag on `cookbook search`, such as `--cuisine indian`.

**Why.** Flags show up in `--help`, and readers never discovered query syntax in testing.

**Rejected.** Query syntax such as `cuisine:indian` inside the search text.
