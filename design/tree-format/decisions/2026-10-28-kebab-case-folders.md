---
id: dc-2b07b62
node: fd-e54a4d
title: "Node folders default to kebab case"
status: accepted
topic: kebab-case-folders
decided_by: alastair
date: 2026-10-28
---

**Decision.** `init` and the `manifestdocs-author` skill create kebab-case folders and node files. `check` accepts existing names.

**Why.** Relative links to kebab-case paths need no escaped spaces.
