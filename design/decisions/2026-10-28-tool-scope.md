---
id: dc-a6c41df
node: fd-622eed
title: "Manifest Docs is shared, living context on top of GitHub"
status: accepted
topic: tool-scope
decided_by: alastair
date: 2026-10-28
---

**Decision.** The tool covers the feature tree, agent context and shared decision records. Anything git, GitHub, `gh` or a Markdown link already does stays out.

**Why.** The earlier hosted version of Manifest grew too large by taking on every feature. A thin layer on GitHub stays small enough to build and change.

**Rejected.** A hosted platform with an MCP server, a web app, a database and auth. Commands for search, history, roadmaps, dates and issue creation, which `rg`, `git log`, GitHub Projects and `gh` already cover.
