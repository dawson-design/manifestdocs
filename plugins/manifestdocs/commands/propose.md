---
description: Propose a change to a feature's requirements in a Manifest Docs tree
argument-hint: <the change, and optionally a node id or path>
allowed-tools: Bash(manifestdocs check:*), Bash(manifestdocs tree:*), Bash(manifestdocs show:*), Bash(manifestdocs context:*), Bash(openssl rand -hex 3)
---

Propose this change in the Manifest Docs tree before any code changes: $ARGUMENTS

Follow "Propose a change" in the `manifestdocs-author` skill. Find the nodes the change touches, and add, change or remove their requirements and scenarios. Give behaviour that no node covers a new node. Run `manifestdocs check` until it exits 0, then show the user the changed requirements. Don't change code until the user approves the proposal.
