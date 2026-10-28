---
id: dc-e2e158c
node: fd-54bf4e
title: "Context follows body links one hop"
status: accepted
topic: linked-docs
decided_by: alastair
date: 2026-10-28
---

**Decision.** `context` loads the local Markdown files the target node's body links to, one hop, in order of first appearance and without duplicates. Links inside loaded docs are listed, not followed. Nodes have no `docs` field.

**Why.** Each link is then recorded in one place. One hop rules out cycles, as llms.txt tooling does by fetching each listed link once. Claude Code and Gemini CLI inline only a dedicated `@` syntax, so a "see also" link doesn't grow their context; here, one hop and the budget limit that growth.

**Rejected.** A `docs` frontmatter list, which repeats the body's links. Following links recursively.
