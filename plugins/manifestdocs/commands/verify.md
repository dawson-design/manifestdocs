---
description: Check a code change against its Manifest Docs node
argument-hint: <node id or path> [base commit]
allowed-tools: Bash(manifestdocs context:*), Bash(manifestdocs show:*), Bash(manifestdocs tree:*), Bash(git diff:*), Bash(git log:*), Bash(git merge-base:*)
---

Verify the current change against the Manifest Docs node and base in `$ARGUMENTS` with the `manifestdocs-verify` skill. If no node was given, use the one this session loaded, or run `manifestdocs tree --depth 2` and ask. Only report: don't edit code, docs or GitHub.
