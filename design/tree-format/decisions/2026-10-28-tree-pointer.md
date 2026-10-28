---
id: dc-e822c7f
node: fd-e54a4d
title: "A .manifestdocs pointer names the tree for the folders under it"
status: accepted
topic: tree-pointer
decided_by: alastair
date: 2026-10-28
---

**Decision.** When walking up finds no `manifestdocs.yaml`, the tool uses the nearest `.manifestdocs/config.yaml` at or above the current folder, whose `tree` is a path relative to that folder. A single repo commits one at its root, such as `tree: docs`. A workspace folder that holds several repos keeps one beside them, such as `tree: cookbook-docs`. Whether the tree is in the working repo follows from the paths and isn't a setting.

**Why.** A single repo needs a tree location that every clone, CI job and sandbox shares without setup. Relative paths work on every machine. A stored mode could contradict the paths.

**Rejected.** Workspaces in `~/.config/manifestdocs/config.yaml`, which every machine has to set up with absolute paths. Searching the repo for its only tree, which guesses when a repo has several. A `single` or `multi` setting.
