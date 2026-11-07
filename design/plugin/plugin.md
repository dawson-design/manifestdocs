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
| Skills | `manifestdocs-context` (run `context` before planning), `manifestdocs-verify` (check a change against its node), `manifestdocs-decide` (spot, draft and confirm decisions, then publish them from a worktree), `manifestdocs-author` (write a valid node, propose a change, or draft a node from existing code), `manifestdocs-plan-issue` (draft linked issues and create them with `gh`), `manifestdocs-capture` (move a decision from an issue into docs) |
| Commands | The [workflow](#workflow) commands, plus `/mfd:stale` and `/mfd:capture` |
| Hooks | Session start: inject a one-level tree outline. Before the session ends: prompt the agent once to review the session for unrecorded decisions. After edits in a docs repo: run `mfd check` |

## Workflow

A change goes through five steps, each with one command. The binary does no step's judgment, and every GitHub write is a visible `git push` or `gh` call the user approves.

| Step | Command | What the agent does | Skill | Milestone |
|---|---|---|---|---|
| Explore | `/mfd:explore` | Loads the node's context before planning | `manifestdocs-context` | M1 |
| Propose | `/mfd:propose` | Changes the nodes' requirements and scenarios before any code, adding a node for behaviour no node covers, for the user to review | `manifestdocs-author` | M1 |
| Apply | `/mfd:apply` | Implements the node one requirement at a time, with a test per scenario | `manifestdocs-context` | M1 |
| Verify | `/mfd:verify` | Reports each requirement, scenario and accepted decision as met, not met or contradicted, plus changed behaviour no requirement covers | `manifestdocs-verify` | M1 |
| Record | `/mfd:record` | Drafts decision records for the choices made along the way | `manifestdocs-decide` | M2 |

Propose grows the tree from the work, so an existing codebase needs no tree up front. Specs change in place, and the tree's `changes` setting decides how they reach the default branch (see [publishing changes](#publishing-changes)). [Node body](../tree-format/node-body/node-body.md) sets the requirement format that propose writes and verify reads. Verify only reports, and it's the check before a change is published.

## Publishing changes

The skills that edit a tree, such as propose and record, publish each change the way the tree's [`changes` setting](../tree-format/tree-format.md#root-marker) names. The setting chooses the path the skills take, and the docs repo's branch rules decide what's allowed, so a team that wants every change reviewed also protects its default branch.

### Requirement: Publish by the changes setting
A skill that edits a tree SHALL open a pull request under `review` and `proposed`, and SHALL push to the docs repo's default branch with no pull request under `direct`.

#### Scenario: Review
- WHEN `changes` is `review` and the agent proposes a spec change or records a decision
- THEN the skill pushes a branch and opens a pull request with `gh` for the team to review

#### Scenario: Proposed
- WHEN `changes` is `proposed` and the agent records a decision
- THEN the skill opens a pull request that the engineer can merge without review, and the record says `proposed`

#### Scenario: Direct
- WHEN `changes` is `direct` and the agent proposes a spec change or records a decision
- THEN the skill commits and pushes to the default branch, and opens no pull request

#### Scenario: Sandboxed run
- WHEN an unattended run records a decision, under any `changes` value
- THEN `mfd decide` writes to `.manifestdocs/decisions/` and the skill pushes nothing

### Requirement: Fall back to a pull request
Under `direct`, a skill MUST open a pull request when it can't push to the default branch.

#### Scenario: Protected branch
- WHEN `changes` is `direct` and the docs repo's branch rules reject the push
- THEN the skill pushes a branch, opens a pull request with `gh`, and tells the user the push was rejected

#### Scenario: Default branch moved
- WHEN `changes` is `direct` and the default branch has moved since the skill's commit
- THEN the skill rebases onto it once and pushes again, and opens a pull request if the rebase conflicts or the push is rejected again

Skills follow the open SKILL.md format so other harnesses can load them. An `AGENTS.md` snippet covers tools without skill support.

The after-edit hook runs `check` on the edited file's tree after a Markdown or `manifestdocs.yaml` edit. It shows the agent the report only when the tree has errors or can't be checked, such as after a bad config edit, so edits to code and to files outside a tree pass silently. Before 1.0, the plugin installs from the local marketplace in the tool's own repo (see [distribution](../distribution/distribution.md)).
