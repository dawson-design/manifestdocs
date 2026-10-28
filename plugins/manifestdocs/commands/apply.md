---
description: Implement a Manifest Docs node's requirements, scenario by scenario
argument-hint: <node id or path>
allowed-tools: Bash(manifestdocs context:*), Bash(manifestdocs tree:*), Bash(manifestdocs show:*)
---

Implement the Manifest Docs node `$ARGUMENTS`. If no node was given, run `manifestdocs tree --depth 2` and ask which node to implement.

1. Load its context with the `manifestdocs-context` skill: run `manifestdocs context $ARGUMENTS --format md --budget 8000` and read the whole output.
2. Plan the work from the node's requirements and scenarios, within its ancestors' constraints and accepted decisions.
3. Implement one requirement at a time. Write a test for each scenario that would fail if the requirement weren't met.
4. When the work is done, run `/manifestdocs:verify` on the node.

If a requirement turns out to be wrong or missing, stop and tell the user. Don't build behaviour the node doesn't describe; propose it with `/manifestdocs:propose` first.
