---
id: fd-ae6745
title: "Decision records"
status: draft
summary: "Records decisions from coding sessions as reviewed files in the docs repo that every engineer and agent reads."
---

# Decision records

This is the shared-memory core of Manifest Docs. Decisions made during coding sessions become records in the docs repo that every engineer and agent on the project reads.

## Record format

One file per decision, in the node's `decisions/` folder, named by date and topic. Separate files avoid merge conflicts when several sessions write at once.

```markdown
---
id: dc-3f1a8c2
node: fd-7c2e41
title: "Search results show 20 recipes per page"
status: accepted          # proposed | accepted | superseded | rejected
topic: search-page-size
decided_by: octocat       # optional
date: 2026-10-28
source: [example-org/cookbook-web#212]
applies_to: [example-org/cookbook-web:src/search/page.ts]
supersedes: null
---

**Decision.** Search shows 20 recipes per page.

**Why.** Readers rarely scroll past 20 results before they refine a search.

**Rejected.** Infinite scroll, which broke the back button on recipe pages.
```

`decided_by` is optional, for teams whose records don't name people. `source` lists issues and pull requests, never harness session IDs. `applies_to` lists the code a decision affects, as `repo:path` text or links for readers, and the tool doesn't check it.

A record captures one choice in a few lines. `check` warns when its body is over 300 estimated tokens, counted as `context` counts them: a choice that needs more is a spec change or an [ADR](#promotion).

## What counts as significant

The `manifestdocs-decide` skill gives agents a short test. Record a decision when the engineer:

- chooses between real alternatives
- fills a gap in a spec or departs from one
- sets a convention that others should follow
- rejects an approach and says why
- learns an outside constraint, such as an API limit or a vendor quirk

Skip personal preferences, which belong in the agent's local memory. Skip details that the code or the commit message already makes plain. Records go to a shared repo, so they never include secrets or personal data.

## Capture

**During a session.** When the engineer makes a call that passes the test, the agent drafts a record and asks the engineer to confirm, edit or drop it. On confirmation, the agent runs `mfd decide`. When a record names a decider, it's the engineer; the agent is never the decider.

When a decision changes what a feature does, the agent also edits the node's requirements in the same docs PR. The spec then states the behaviour, and the record keeps the why and the rejected options. A record that supersedes another names the old choice under **Rejected**, with the reason it was dropped.

A decision that applies to several nodes goes on their nearest common ancestor, which `context` loads for each of them. One that applies to no node goes on the root node.

**On demand.** `/manifestdocs:record` asks the agent to review the session and propose any decisions it missed, before the transcript is compacted or closed. When nothing passes the test, the agent says there's nothing to record. No hook prompts for this, because Claude Code has no hook that can keep a session going as it ends.

## Status

`decide` sets the status from the repo's `decisions.merge` setting. Under `review`, it writes `accepted`, because the PR review is the approval. Under `proposed`, it writes `proposed`, and a later one-line PR accepts the record. Unattended runs always write `proposed`, and under `review` the person who opens their PR changes the records they endorse to `accepted` in that PR. A record gets the same status whether an agent or a person drafted it; `decided_by` and git history show who. Reviewers reject a record by setting `rejected` with the reason and merging it, not by closing the PR, so the record still stops the same idea from coming back.

## Input

`mfd decide --file <path>` reads one record in the on-disk format, and `--file -` reads it from stdin. The tool fills in `id`, `date` and `status`, and rejects a supplied `id`, an unquoted `title`, unknown fields and a status the repo setting forbids. It reports every validation error in one pass, each with the field, line and fix, and exits non-zero; `--json` returns the errors as an array. `--dry-run` prints the final record and its path without writing. Rerunning with the same node, topic and body exits 0 and prints the existing path, and a different body fails with a suggestion to use `supersedes`. `--unattended` marks factory runs, because agent shells often have no terminal to detect.

## Write path

`decide` writes the record and prints its path. When the record has `supersedes`, it also changes the older record's `status` line to `superseded`, and prints that path too. A record that supersedes one from the same day gets the next free name, such as `2026-10-28-search-page-size-2.md`. It never branches or commits, and it warns when the docs clone it writes to has uncommitted changes, or commits that aren't on its default branch, `origin/HEAD`. A fresh branch from the default branch, such as the skill's worktree, gets no warning.

| Setting | Interactive session | Sandboxed factory run |
|---|---|---|
| Where `mfd decide` writes | The node's `decisions/` folder in a git worktree of the docs repo, on a new branch from the default branch. The `manifestdocs-decide` skill creates the worktree, so the main docs clone never switches branches | `.manifestdocs/decisions/` in the code repo's working directory, because the docs mount is read-only |
| What happens next | The skill commits, pushes and opens a PR with `gh`, then removes the worktree. With `merge: review` the team reviews the PR; with `merge: proposed` the engineer can merge it directly | The factory host moves the files into a docs branch, and a person opens the PR |
| Confirmation | The team reviews the PR, or a later review flips `proposed` to `accepted` | The same, after the run |

When the tool creates `.manifestdocs/decisions/`, it writes a `.gitignore` containing `*` inside it, so drafts never get committed to a code repo, while a [pointer](../tree-format/pointer/pointer.md) beside it can be. `--out <dir>` overrides the folder.

When the tree is in the working repo, the skill creates the worktree from that repo, and the record still gets its own PR, apart from the code.

The personal factory can use `merge: proposed` for speed. A team whose changes need review before they take effect uses `merge: review`.

## Read path

Other sessions see a record once its PR merges and their docs clone fetches. `mfd context` includes accepted decisions for the node and its ancestors. It also lists proposed decisions, marked as proposed. Superseded and rejected records are left out; `mfd decisions` lists them. A rejected record still prevents the next session from proposing the same idea without a new reason. It never expires: revisiting it takes a new record that supersedes it and gives the new reason.

Each decision takes one line in `context`: its ID, status and title, and the first sentence of its **Why.** paragraph. When a record supersedes another, its line ends with the older record's ID and title, as `Supersedes dc-1a7f3b4: <title>`. The tool reads these from the older file, so nothing is stored twice. Only the immediate predecessor appears, and `mfd show <dc-id>` prints a record in full with the chain it supersedes. Each node's decisions go into the budget right after the node, newest first, so trimming drops the oldest decisions of the farthest ancestors first. A record's length doesn't change its cost in context, so a node can gather many decisions without crowding out its spec.

## Promotion

When a decision changes an approved spec or spans several nodes, the agent drafts an ADR from the record, in the docs repo's own ADR format. Once the ADR merges, the record links to it and its status becomes `superseded`.

