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
| `mfd show <id or path>` | One node in full, with supporting docs listed. From M2, a `dc-` ID shows one decision record in full, with the chain of records it supersedes | No |
| `mfd context <id or path> [--out path]` | Node and its ancestors in full, linked docs, child summaries | No |
| `mfd context <issue-ref>` | Issue, summaries of its parent chain, linked node context | Yes |
| `mfd check [path] [--strict] [--offline]` | Validate the tree that contains `path`, or the current folder; see [checks](../checks/checks.md). `--offline` skips the GitHub rules without calling `gh` | No |
| `mfd work <id>` | Work items linked to a node and their sub-issues, with open or closed state | Yes |
| `mfd stale` | Report open issues whose node changed after their `spec-ref` | Yes |
| `mfd decide` | Write a decision record from a file or stdin; see [decision records](../decision-records/decision-records.md) | No |
| `mfd decisions [id]` | List decision records for a node and its ancestors, or for the whole tree | No |
| `mfd init [title]` | Create `manifestdocs.yaml` and a root node, named after the title in kebab case | No |

Commands that need GitHub fail with a clear message when `gh` is missing or unauthenticated. All other commands still work.

The read commands, `tree`, `show`, `context`, `check` and `decisions`, take `--ref <ref>` to read a branch, tag or commit in the docs clone instead of the [version they read by default](../tree-format/tree-format.md#which-version-the-tool-reads): the working tree for `tree`, `show` and `check`, and from another repo `origin/HEAD` for `context` and `decisions`. `--ref` is an error for a tree outside git. A path argument names a file or folder in the version read, so a path that exists only in that commit works, and one that exists only on disk isn't found. `context --out <path>` writes the context to that file instead of stdout, creating its folders, and prints `Wrote <path>` to stderr.

Paths on the command line and in output are relative to the current folder, so a path one command prints works as another's argument. `check` exits 1 when the tree has errors, or any findings with `--strict`, and `show` exits 1 when the node's frontmatter has errors, which it prints. Every command exits 2 when it can't run, such as when there's no tree or an argument names no single node.

## Output

- **Text** for people, the default in a terminal.
- **`--json`** for scripts and tooling, with a documented, versioned schema. See [JSON output](#json-output).
- **`--format md --budget <tokens>`** for agents; see [agent context](../context/context.md).

## JSON output

With `--json`, every command prints one JSON object to stdout, in schema version 1:

```json
{ "schema": 1, "command": "check", "data": { "nodes": 11, "findings": [] } }
```

- A command that can't run prints `{ "schema": 1, "command": "<name>", "error": "<message>" }` instead of `data`. Exit codes stay the same as for text.
- Keys are snake case. A field with no value is `null`, never left out, and a list with no items is `[]`.
- A text field that's known but empty is `""`, and `null` means the value is unknown, such as when a node's frontmatter has errors. `body` is the trimmed text, so a node with no body has `""`.
- Paths are relative to the current folder, as in text output. IDs are always full length.
- A finding is `{ "file", "line", "severity", "code", "message" }`. `code` is the problem's name in snake case, such as `broken_link`, and `message` is the text line's message.
- Adding a field keeps version 1. Removing or renaming a field, or changing its type, raises the version.

| Command | `data` |
|---|---|
| `check` | `{ nodes, findings }` |
| `tree` | `{ nodes: [{ id, title, status, file, parent }] }` in outline order, with `parent` an ID or `null` |
| `show` | `{ id, title, status, summary, file, parent, repos, work, superseded_by, body, supporting_docs, findings }`, or a record's fields for a `dc-` ID |
| `context` | `{ target, version, pieces: [{ kind, heading, path, text }], omitted: [{ path, tokens }], tokens, budget }` |
| `decide`, `decisions`, `work`, `stale` | Set by each command's milestone, under these rules |
