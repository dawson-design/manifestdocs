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
| `mfd context <issue-ref> [--out path]` | Issue, summaries of its parent chain, linked node context. `<issue-ref>` is `owner/name#number` | Yes |
| `mfd check [path] [--strict] [--offline]` | Validate the tree that contains `path`, or the current folder; see [checks](../checks/checks.md). `--offline` skips the GitHub rules without calling `gh` | No |
| `mfd work <id>` | Work items linked to a node and their sub-issues, with open or closed state | Yes |
| `mfd stale` | Report open issues whose node changed after their `spec-ref` | Yes |
| `mfd decide` | Write a decision record from a file or stdin; see [decision records](../decision-records/decision-records.md). It reads and writes the docs clone's working tree, and takes no `--ref` | No |
| `mfd decisions [id] [--ref ref]` | List decision records for a node and its ancestors, or for the whole tree | No |
| `mfd init [title]` | Create `manifestdocs.yaml` and a root node, named after the title in kebab case | No |

Commands that need GitHub fail with a clear message when `gh` is missing or unauthenticated. All other commands still work.

The read commands, `tree`, `show`, `context`, `check`, `decisions` and `work`, take `--ref <ref>` to read a branch, tag or commit in the docs clone instead of the [version they read by default](../tree-format/tree-format.md#which-version-the-tool-reads): the working tree for `tree`, `show` and `check`, and from another repo `origin/HEAD` for `context`, `decisions` and `work`. `stale` takes no `--ref`, and reads the version `work` reads. `--ref` is an error for a tree outside git. A path argument names a file or folder in the version read, so a path that exists only in that commit works, and one that exists only on disk isn't found. `context --out <path>` writes the context to that file instead of stdout, creating its folders, and prints `Wrote <path>` to stderr.

Paths on the command line and in output are relative to the current folder, so a path one command prints works as another's argument. `check` exits 1 when the tree has errors, or any findings with `--strict`, `show` and `context` exit 1 when the node's frontmatter has errors, which they print, `work` exits 1 when it can't read an issue on GitHub, which it marks, as `context <issue-ref>` does for the issue's parents, and `stale` exits 1 when it reports an issue that's stale or that it can't check. Every command exits 2 when it can't run, such as when there's no tree or an argument names no single node.

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
| `show` | `{ id, title, status, summary, file, parent, repos, work, superseded_by, body, supporting_docs, findings }`. For a `dc-` ID, `{ id, node, title, status, topic, date, decided_by, source, applies_to, supersedes, superseded_by, body, file, findings, chain }`, where `superseded_by` is the ID of the first record, in path order, that supersedes it and `chain` lists the records it supersedes, newest first, each with the fields before `findings` and superseded by the record before it. With frontmatter errors, the fields other than `id`, `superseded_by` and `file` are `null` and `chain` is `[]` |
| `context` | `{ target, version, pieces: [{ kind, heading, path, text }], omitted: [{ kind, path, tokens }], tokens, budget }`. `target` is the node's ID or the issue. `version` is `null` outside git, else `{ ref, commit, date, uncommitted, default_branch }`, with `ref` `null` on a detached HEAD and `default_branch` the branch `origin/HEAD` names, only for the working tree. `pieces` are what went in, in print order, and `omitted` what didn't, in fill order. `kind` is `issue`, `parent_work`, `target`, `parent`, `ancestor`, `project`, `decision`, `linked_doc`, `linked_node`, `other_link` or `child`. `heading` is the piece's heading, or its section's for a list item, and `text` the trimmed Markdown under it. `path` is the piece's file, or the path or URL a link lists, and `null` for the issue and parent work. `tokens` is the text output's estimate. A target with frontmatter errors prints them as `error` and exits 1. With `--out`, the JSON goes to the file. `--json` can't be used with `--format` |
| `decide` | `{ outcome, id, status, file, record, superseded: { id, file, record }, errors: [{ file, line, field, severity, code, message, fix }], warnings: [{ code, message }] }`. `outcome` is `written`, `dry_run`, `unchanged` or `invalid`. `record` is the text written, or that would be written on a dry run, or `null`. `superseded` is the record marked superseded, with its new text, or `null`. An error's `file` is `null` for stdin |
| `decisions` | `{ records: [{ id, node, title, status, topic, date, supersedes, file }] }` grouped by node, with `node` the node whose `decisions/` folder holds the record |
| `work` | `{ id, file, work: [issue], findings }`, with `work` `null` when the node's frontmatter has errors. An issue is `{ issue, found, error, title, state, issue_type, parents, sub_issues_done, sub_issues_total, listed_by, sub_issues: [issue] }`. `parents` lists the issue's parents nearest first, up to the top. When `found` is `false`, `error` says why, `title`, `state`, `issue_type`, `sub_issues_done` and `sub_issues_total` are `null`, and `sub_issues` is `[]`. `parents` is still filled in for a sub-issue, from the issues above it, and `issue` is `null` for a sub-issue GitHub listed but couldn't return, or for the sub-issues past a page cursor GitHub repeated. `listed_by` is the ID of a deeper node that lists the issue in its own `work`. That node takes the issue's sub-issues, so they aren't read: `sub_issues` is `[]`, and `sub_issues_done` and `sub_issues_total` are `null` |
| `init` | `{ files, id }`: the files created, `manifestdocs.yaml` then the root node, and the root node's ID |
| `stale` | `{ open, stale: [{ issue, title, node, file, spec_ref, created_at, since, changed_in, changes: [{ path, added, removed }] }], not_checked: [{ issue, title, node, file, error }] }`. `open` counts the open linked issues it compared. `node` is a node ID and `file` its node file. `since` and `changed_in` are `{ commit, date }`, with full commit IDs: the version the issue was planned against, `null` when no docs commit is older than the issue, and the node's latest change. `added` and `removed` are `null` for a binary file. In `not_checked`, `issue` is `null` for a sub-issue GitHub listed but couldn't return, and `title` is `null` when `gh` couldn't read the issue |
