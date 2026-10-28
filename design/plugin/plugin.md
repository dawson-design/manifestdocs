---
id: fd-fd80a6
title: "Claude Code plugin"
status: draft
summary: "Skills, commands and hooks that call the binary from Claude Code and other harnesses that load skills."
---

# Claude Code plugin

The plugin contains no logic. Each skill checks for the binary and calls it through the shell.

| Part | Contents |
|---|---|
| Skills | `manifestdocs-context` (run `context` before planning), `manifestdocs-decide` (spot, draft and confirm decisions, then open their PR from a worktree), `manifestdocs-author` (write a valid node), `manifestdocs-plan-issue` (draft linked issues and create them with `gh`), `manifestdocs-capture` (move a decision from an issue into docs) |
| Commands | `/manifestdocs:context`, `/manifestdocs:decide`, `/manifestdocs:stale`, `/manifestdocs:capture` |
| Hooks | Session start: inject a one-level tree outline. Before the session ends: prompt the agent once to review the session for unrecorded decisions. After edits in a docs repo: run `mfd check` |

Skills follow the open SKILL.md format so other harnesses can load them. An `AGENTS.md` snippet covers tools without skill support.
