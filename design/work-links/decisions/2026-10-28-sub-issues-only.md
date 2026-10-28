---
id: dc-2ed3266
node: fd-0e0a4e
title: "The work hierarchy is GitHub sub-issues"
status: accepted
topic: sub-issues-only
decided_by: alastair
date: 2026-10-28
---

**Decision.** The tool reads GitHub sub-issues at any depth, and treats issues as flat when a repo has none.

**Why.** Sub-issues are GitHub's own hierarchy, and the tool is a layer on GitHub.

**Rejected.** Hierarchies from task lists, labels and Projects fields, and adapters for other trackers such as Jira or Shortcut.
