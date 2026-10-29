---
description: Review this session for decisions and record the ones the engineer confirms
argument-hint: [node id or path]
allowed-tools: Bash(manifestdocs decide:*), Bash(manifestdocs decisions:*), Bash(manifestdocs tree:*), Bash(manifestdocs show:*), Bash(manifestdocs context:*), Bash(git worktree:*)
---

Review this session for decisions worth recording with the `manifestdocs-decide` skill. If `$ARGUMENTS` names a node, start with decisions about that node.

For each choice that passes the skill's test, draft a record and ask the engineer to confirm, edit or drop it. Write and open a pull request only for the ones they confirm, following the skill's write path. If nothing passes the test, say there's nothing to record.
