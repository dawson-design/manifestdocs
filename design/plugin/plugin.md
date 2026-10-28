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
| Skills | `manifestdocs-context` (run `context` before planning), `manifestdocs-verify` (check a change against its node), `manifestdocs-decide` (spot, draft and confirm decisions, then open their PR from a worktree), `manifestdocs-author` (write a valid node, propose a change, or draft a node from existing code), `manifestdocs-plan-issue` (draft linked issues and create them with `gh`), `manifestdocs-capture` (move a decision from an issue into docs) |
| Commands | The [workflow](#workflow) commands, plus `/manifestdocs:stale` and `/manifestdocs:capture` |
| Hooks | Session start: inject a one-level tree outline. Before the session ends: prompt the agent once to review the session for unrecorded decisions. After edits in a docs repo: run `mfd check` |

## Workflow

A change goes through five steps, each with one command. The binary does no step's judgment, and every GitHub write is a visible `gh` call the user approves.

| Step | Command | What the agent does | Skill | Milestone |
|---|---|---|---|---|
| Explore | `/manifestdocs:explore` | Loads the node's context before planning | `manifestdocs-context` | M1 |
| Propose | `/manifestdocs:propose` | Changes the nodes' requirements and scenarios before any code, adding a node for behaviour no node covers, for the user to review | `manifestdocs-author` | M1 |
| Apply | `/manifestdocs:apply` | Implements the node one requirement at a time, with a test per scenario | `manifestdocs-context` | M1 |
| Verify | `/manifestdocs:verify` | Reports each requirement, scenario and accepted decision as met, not met or contradicted, plus changed behaviour no requirement covers | `manifestdocs-verify` | M1 |
| Record | `/manifestdocs:record` | Drafts decision records for the choices made along the way | `manifestdocs-decide` | M2 |

Propose grows the tree from the work, so an existing codebase needs no tree up front. Specs change in place on a docs branch, and the pull request is the review. [Node body](../tree-format/node-body/node-body.md) sets the requirement format that propose writes and verify reads. Verify only reports, and it's the check before a pull request.

Skills follow the open SKILL.md format so other harnesses can load them. An `AGENTS.md` snippet covers tools without skill support.

The after-edit hook runs `check` on the edited file's tree after a Markdown or `manifestdocs.yaml` edit. It shows the agent the report only when the tree has errors or can't be checked, such as after a bad config edit, so edits to code and to files outside a tree pass silently. Before 1.0, the plugin installs from the local marketplace in the tool's own repo (see [distribution](../distribution/distribution.md)).
