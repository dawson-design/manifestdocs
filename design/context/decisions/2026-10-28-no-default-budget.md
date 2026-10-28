---
id: dc-935a93c
node: fd-54bf4e
title: "Nothing is trimmed without --budget"
status: accepted
topic: no-default-budget
decided_by: alastair
date: 2026-10-28
---

**Decision.** Without `--budget`, `context` trims nothing. The output always ends with its estimated size, and the plugin's hooks and skills always pass a budget.

**Why.** Context packers that people run by hand, such as Repomix and code2prompt, don't trim by default, and any default number would be arbitrary across models. The plugin covers the automatic case, where Aider, Claude Code and Codex cap context by default.

**Rejected.** A default budget.
