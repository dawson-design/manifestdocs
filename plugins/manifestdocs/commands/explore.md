---
description: Explore a Manifest Docs node's context before planning
argument-hint: <node id or path>
allowed-tools: Bash(manifestdocs context:*), Bash(manifestdocs tree:*)
---

Load the context for the Manifest Docs node `$ARGUMENTS` with the `manifestdocs-context` skill: run `manifestdocs context $ARGUMENTS --format md --budget 8000` and read the whole output before planning.

If no node was given, run `manifestdocs tree --depth 2` and ask which node to load.
