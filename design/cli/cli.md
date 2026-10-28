---
id: fd-8ff5b9
title: "CLI"
status: draft
summary: "The commands, and their text and JSON output."
---

# CLI

## Commands

| Command | Purpose | Needs GitHub |
|---|---|---|
| `mfd tree [path] [--depth n]` | Outline of nodes with status | No |
| `mfd show <id or path>` | One node in full, with supporting docs listed | No |
| `mfd context <id or path>` | Node in full, the root node, linked docs, ancestor and child summaries | No |
| `mfd context <issue-ref>` | Issue, summaries of its parent chain, linked node context | Yes |
| `mfd check [path]` | Validate the tree that contains `path`, or the current folder; see [checks](../checks/checks.md) | No |
| `mfd work <id>` | Work items linked to a node and their sub-issues, with open or closed state | Yes |
| `mfd stale` | Report open issues whose node changed after their `spec-ref` | Yes |
| `mfd decide` | Write a decision record from a file or stdin; see [decision records](../decision-records/decision-records.md) | No |
| `mfd decisions [id]` | List decision records for a node and its ancestors, or for the whole tree | No |
| `mfd init [title]` | Create `manifestdocs.yaml` and a root node, named after the title in kebab case | No |

Commands that need GitHub fail with a clear message when `gh` is missing or unauthenticated. All other commands still work.

Paths on the command line and in output are relative to the current folder, so a path one command prints works as another's argument. `check` exits 1 when the tree has errors, and `show` exits 1 when the node's frontmatter has errors, which it prints. Every command exits 2 when it can't run, such as when there's no tree or an argument names no single node.

## Output

- **Text** for people, the default in a terminal.
- **`--json`** for scripts and tooling, with a documented, versioned schema.
- **`--format md --budget <tokens>`** for agents; see [agent context](../context/context.md).
