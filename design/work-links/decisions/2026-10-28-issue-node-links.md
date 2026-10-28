---
id: dc-f83e967
node: fd-0e0a4e
title: "Nodes list top-level issues, and sub-issues inherit the link"
status: accepted
topic: issue-node-links
decided_by: alastair
date: 2026-10-28
---

**Decision.** A node lists its top-level issues in `work`, and their sub-issues link through the hierarchy. An issue outside that hierarchy names its node with a `feature:` line in its body. `spec-ref` is optional, and the tool falls back to the docs commit from when the issue was created.

**Why.** Each link is recorded in one place, and existing issues work without edits to bodies other people wrote.

**Rejected.** A label per node, which clutters a repo's label list. Projects fields, which are stored outside the issue, need a project-scoped token and differ from project to project.
