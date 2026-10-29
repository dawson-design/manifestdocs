---
description: Capture the decisions in a GitHub issue's thread into a Manifest Docs tree
argument-hint: <owner/name#number>
allowed-tools: Bash(manifestdocs context:*), Bash(manifestdocs decide:*), Bash(manifestdocs decisions:*), Bash(manifestdocs tree:*), Bash(manifestdocs show:*), Bash(gh issue view:*), Bash(git worktree:*)
---

Capture the decisions made in the GitHub issue `$ARGUMENTS` with the `manifestdocs-capture` skill. If no issue was given, ask for one as `owner/name#number`.

Read the thread, draft a record for each choice that passes the skill's test, and ask the user to confirm, edit or drop each one. Write and commit only the confirmed ones, on a docs branch in a worktree, and leave the pull request for a person to open. If nothing passes the test, say there's nothing to capture.
