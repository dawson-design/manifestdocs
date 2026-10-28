# PRD: Manifest Docs (`mfd`)

**Owner:** Alastair Dawson
**Date:** October 28, 2026
**Status:** Draft
**Domain:** manifestdocs.ai

## 1. Summary

Manifest Docs is a command-line tool that adds shared, living context on top of GitHub and GitHub Enterprise. It turns a folder of Markdown in a docs repo into a feature tree that agents and people can walk, and its files read as ordinary pages on GitHub without the tool. Each node is a folder with the requirements and decisions for one feature. The tool links nodes to work in GitHub Issues and flags issues whose requirements changed after planning. It also assembles focused context for coding agents and records the decisions engineers make during coding sessions, so every person and agent on the project shares them.

It ships as one Rust binary (`manifestdocs`, alias `mfd`) and a thin Claude Code plugin. Git stores the tree and its history. GitHub hosts and reviews it. The binary reads from GitHub and never writes to it, and anything that git, GitHub, `gh` or a Markdown link already does stays out of the tool. There is no server, database or account.

## 2. Problem

Teams keep two kinds of knowledge in different places, and agents need both.

- **Decisions live in docs.** PRDs, specs, design docs and ADRs sit in a separate docs repo. They change through reviewed commits, so their history is clear.
- **Work lives in issues.** Issues, often grouped under parents such as epics and initiatives, track who does what and when. No single issue, even a top-level one, can hold all the product and design decisions behind it. Edits to issue bodies and decisions made in comments leave no reviewable history.

Agents fall into the gap between the two. An issue gives them the task without the reasoning. The docs repo gives them the reasoning without a way to find the part that matters. When a spec changes after someone plans an issue, nothing tells the agent or the reviewer. Decisions made in issue threads never reach the docs, so the next agent repeats the debate.

Coding sessions add a third gap. An engineer makes real decisions while pairing with an agent: which approach to take, which convention to follow, which idea failed and why. Claude often saves these as memories, but that memory is local to one user. The next engineer, and every other agent, starts without it. The decision exists only in a transcript nobody will read again.

The earlier version of Manifest tried to solve this with a hosted platform: an MCP server, a web app, a database and auth. It grew too large. The core ideas, context as a feature tree and decisions written back as shared memory, work without any of that.

## 3. Goals

1. Give agents the right context for an issue in one command, within a token budget.
2. Keep requirements and decisions in versioned docs, and keep work in GitHub Issues.
3. Show when an issue's requirements changed after it was planned.
4. Turn significant decisions from coding sessions, issue threads and agent results into shared, reviewed records in the docs repo.
5. Work offline inside a sandbox with no credentials.
6. Run in any agent harness that has a shell.
7. Keep the docs tree readable on GitHub, as ordinary pages, for people who don't have the tool.

## 4. Non-goals

- Replacing GitHub Issues, Projects or any work tracker.
- Supporting work trackers other than GitHub Issues. The tool is a layer on GitHub.
- Writing to GitHub. Agents create issues, comments and pull requests with `gh`, so the harness's permission rules see each write.
- Tracking dates, order, priority or delivery. GitHub Projects covers them.
- Search and history commands. `rg` and `git log --follow` cover them.
- Managing images, diagrams, style guides or external references. They're Markdown links.
- Hosting docs, rendering a website or providing a web editor.
- Running an MCP server in the first release.
- Storing data outside git and GitHub.
- Managing agent working memory for a single task. Dex, beads and Claude Code's built-in tasks already cover that.

## 5. Users

| User | Need |
|---|---|
| Coding agent | Read the requirements, decisions and work hierarchy for its issue before planning |
| Solo developer | Keep a personal software factory's agents aligned with game rules and design docs |
| Engineering team | Keep specs, ADRs and stories consistent across several code repos and a shared docs repo |
| Factory tooling | Build `.context/` for each agent run, gate docs PRs in CI, report stale issues |
| Coding agent in a session | Notice when the engineer makes a significant decision and propose a record of it |
| Docs-lane agent | Draft ADRs and spec updates from decisions captured in issues and PR results |

## 6. Concepts

### Two trees, linked

| | Feature tree | Work tree |
|---|---|---|
| Contents | What to build and why | Who does what, and when |
| Shape | Area > feature > sub-feature | Flat issues, or GitHub sub-issues at any depth, such as initiative > epic > task |
| Stored in | Folders in the docs repo | GitHub Issues |
| Lifespan | As long as the product | Until the work ships |
| Change history | Git commits through reviewed PRs | Untracked edits and comments |

The trees don't map one to one. A top-level work item can touch several areas, and an issue can change one acceptance criterion. Manifest Docs models the links between them rather than forcing one shape onto the other.

### Three layers

| Layer | Contents | Tool |
|---|---|---|
| Decisions | Requirements, design, ADRs | Manifest Docs |
| Work | Issues and their parents | GitHub Issues |
| Agent working memory | One issue broken into steps | Dex, beads or built-in agent tasks |

### Personal memory and shared decisions

| | Personal memory | Shared decisions |
|---|---|---|
| Example | "Prefers small PRs." "Uses Zed." | "Dedup window defaults to ten minutes because producers retry for up to eight." |
| Scope | One person | The project |
| Stored in | The agent's local memory | Decision records in the docs repo |
| Review | None | A PR, or merged as `proposed` and confirmed later |
| Tool | The harness's memory feature | Manifest Docs |

A decision record is lighter than an ADR. It captures one choice in a few lines, attached to a feature node. When a decision changes an approved spec or cuts across several features, the record becomes the starting draft for an ADR.

### Rules the tool supports

- Issues point to docs and never restate them.
- A decision is real once it merges into the docs repo. Comments and sessions can discuss it.
- People make decisions. Agents notice and draft them, and the engineer confirms.
- The binary never calls a model. Agents do the judgment; Manifest Docs stores, validates and retrieves.
- The binary reads from GitHub and never writes to it. Agents write with `gh`.
- Each link is recorded in one place. A feature node lists its top-level work items, and their sub-issues inherit the link.
- An issue can record the docs commit it was planned against. Without one, the tool uses the docs commit from when the issue was created.
- Images, diagrams, style guides and external references are Markdown links, not features.

## 7. Requirements

### 7.1 Tree format

**Root marker.** A `manifestdocs.yaml` file marks the root of a tree. It can go in any folder of a docs repo, and one repo can contain several trees. A `manifestdocs.yaml` inside an existing tree starts a separate tree, and the outer tree skips that folder. The tool locates the root by walking up from the current directory, then through the user configuration below.

```yaml
# manifestdocs.yaml
name: event-bus
github:
  host: github.com
  docs_repo: example-org/eeb-docs
decisions:
  merge: review          # review | proposed
statuses: [draft, approved, superseded]
```

Only `name` is required. Without a `github` block, the tree commands still work and the work commands are off.

**User configuration.** Engineers usually work in a code repo cloned next to the docs repo, and walking up from there never reaches the root. A user-level file maps workspace folders to tree roots.

```yaml
# ~/.config/manifestdocs/config.yaml
workspaces:
  - path: ~/work/eeb             # sessions anywhere under this folder
    tree: ~/work/eeb/eeb-docs    # the folder with manifestdocs.yaml
```

**Which version the tool reads.** Inside the docs clone, the tool reads the working tree, so authors see their edits and `check` validates the checked-out branch. The context header then shows the branch and any uncommitted changes, and warns when the clone isn't on its default branch. From any other repo, the tool reads the docs repo's default branch, which it resolves from `origin/HEAD`, so a docs clone left on a feature branch doesn't change an agent's context. When `origin/HEAD` is unset, the tool fails and prints the command that sets it, `git remote set-head origin --auto`, rather than guessing a branch name. The tool never fetches, so the header shows the commit and its date, and the ref is as current as the clone's last fetch. A `--ref` flag overrides both.

**Nodes.** A folder under the root is a feature node when it contains a Markdown file named after the folder, such as `recipes/recipes.md`, with an `id` in its frontmatter. Because each node file has its own name, editor tabs and search results show which feature a file belongs to. The root node is named after the tree's `name` in `manifestdocs.yaml`, such as `event-bus.md`, because a clone's folder name isn't fixed. The root node is optional: without one, top-level nodes have no parent, `context` leaves out the project section, and `check` warns. A node's parent is the nearest ancestor folder that is a node. Folders in between are passed through, so a tree can follow a repo's existing folder layout. Node folder names default to kebab case, such as `key-strategies/`, so relative links need no escaped spaces. `init` and the `manifestdocs-author` skill create folders and node files that way, and `check` accepts existing names. When a folder is renamed without its node file, `check` reports the mismatch, because the node would otherwise drop out of the tree. A `decisions/` subfolder contains the node's decision records (7.9).

**Ownership.** A node owns every file under its folder, except files under a deeper node's folder or a nested tree. Those files are its supporting docs: `show` lists them, and `stale` watches them. `context` loads only the ones the node links to (7.5).

For example, `platform/dedup/dedup.md`:

```markdown
---
id: fd-7c2e41
title: Universal De-Duplicator
status: approved
summary: Drops repeat events across producers using a shared key and a time window.
repos: [example-org/eeb-dedup]
work: [example-org/eeb-docs#41]
superseded_by: null
---

Requirements, acceptance criteria and open decisions for the node, with links
to supporting docs such as the [key rules](dedup-keys.md) and the
[window ADR](../../adr/2026-09-dedup-window.md).
```

| Field | Required | Rule |
|---|---|---|
| `id` | Yes | `fd-` plus 6 random hex characters, unique within the tree |
| `title` | Yes | Plain name |
| `status` | Yes | One of the statuses in `manifestdocs.yaml` |
| `summary` | Yes | One sentence, used in ancestor and child summaries |
| `repos` | No | Code repos that implement the node |
| `work` | No | Top-level work items for the node. Their sub-issues link through the hierarchy |
| `superseded_by` | When superseded | ID of the replacing node |

**Links.** Images, diagrams, style guides and external references, such as a design file or a platform's guidelines, are ordinary Markdown links in the node body. Mermaid diagrams go inline. Links between nodes use relative paths, so they work on GitHub. `check` confirms that local links resolve. The tool doesn't fetch or track external links.

**Stable IDs.** Links use node IDs. Output shows paths for readability. When a folder moves, links stay valid and `check` reports the new path. IDs are a prefix plus lowercase hex from the operating system's random source: `fd-` and 6 characters for nodes, `dc-` and 7 for decision records. The tool generates a new ID when one matches an existing ID in the tree, and `check` reports duplicates, such as two branches that picked the same ID. When reading, the tool accepts IDs of 4 or more characters. Records are superseded rather than deleted, so their IDs stay in the tree.

### 7.2 Links to work

- A feature node lists its top-level work items in `work`. Their sub-issues link to the node through the hierarchy, so their bodies need no link.
- When a deeper node lists a sub-issue in its own `work`, the deeper node applies to that issue and its sub-issues.
- An issue outside that hierarchy names its node with a line in its body: `feature: fd-7c2e41`.
- An issue can record the docs commit it was planned against: `spec-ref: <sha>`. Without one, the tool uses the last commit on the docs repo's default branch before the issue was created, so existing issues work without edits.
- The tool reads both directions and never writes a second copy of a link.
- Manifest Docs doesn't use labels or Projects fields for node links. A label per node clutters a repo's label list. Projects fields are stored outside the issue, need a project-scoped token, and differ from project to project.

### 7.3 Work hierarchy

Manifest Docs reads GitHub's sub-issues, at any depth. In a repo without them, every issue stands alone. Level names such as initiative, epic or task are display labels only: the tool reads them from issue types when present and never requires them. `context` and `work` show the chain of parents and children that sub-issues return.

### 7.4 Commands

| Command | Purpose | Needs GitHub |
|---|---|---|
| `mfd tree [path] [--depth n]` | Outline of nodes with status | No |
| `mfd show <id or path>` | One node in full, with supporting docs listed | No |
| `mfd context <id or path>` | Node in full, the root node, linked docs, ancestor and child summaries | No |
| `mfd context <issue-ref>` | Issue, summaries of its parent chain, linked node context | Yes |
| `mfd check` | Validate the tree (see 7.6) | No |
| `mfd work <id>` | Work items linked to a node and their sub-issues, with open or closed state | Yes |
| `mfd stale` | Report open issues whose node changed after their `spec-ref` | Yes |
| `mfd decide` | Write a decision record from a file or stdin (7.9) | No |
| `mfd decisions [id]` | List decision records for a node and its ancestors, or for the whole tree | No |
| `mfd init` | Create `manifestdocs.yaml` and a root node | No |

Commands that need GitHub fail with a clear message when `gh` is missing or unauthenticated. All other commands still work.

### 7.5 Output

- **Text** for people, the default in a terminal.
- **`--json`** for scripts and tooling, with a documented, versioned schema.
- **`--format md --budget <tokens>`** for agents. The budget is approximate. The tool estimates tokens as characters divided by three and reserves 10% headroom. That overestimates for tokenizers near four characters per token, and allows for newer tokenizers, such as recent Claude models', that produce more tokens for the same text. It fills the budget in this order: the target node in full, the root node in full when the tree has one, linked docs, the issue, parent summaries, ancestor summaries, then child summaries. The root node goes in full because it states the rules that apply across the project, such as a game's core rules or a service's delivery guarantee. The tool trims from the end and lists what it left out by path, so an agent can read it. Without `--budget`, nothing is trimmed. Either way, the output ends with its estimated size, and the plugin's hooks and skills always pass a budget.

**Linked docs.** `context` follows links one hop from the target node's body. Local Markdown files load in order of first appearance, without duplicates, skipping the target and root nodes. Links inside code spans and fenced blocks don't count. Links to other nodes appear as one-line summaries, and other links, such as images and external URLs, are listed by path. Links inside loaded docs are listed, not followed, so there are no cycles.

Example `context` output in agent format:

```markdown
# Context: example-org/eeb-docs#112 (docs origin/main @ 3f9a1c2, 2026-10-27)

## Issue
Add a configurable dedup window per topic. Acceptance criteria: ...

## Parent work
- #41 Universal De-Duplicator (open)
  - #58 Window configuration (open, 3 of 5 done)

## Feature: Universal De-Duplicator (fd-7c2e41, approved)
Path: platform/dedup
<full node body>

## Project: Event Bus (fd-3e9d0a, approved)
<full root node body>

## Decisions
- dc-3f1a8c2 (accepted): Default dedup window is ten minutes. Producers retry for up to eight.
- dc-8b07e15 (proposed): Window is set per topic, not per producer.

## Linked docs
### adr/2026-09-dedup-window.md
<full doc>

## Around this feature
- Parent: Platform services: <summary>
- Child: Key strategies (fd-91ab5d, draft): <summary>

Omitted for budget: platform/dedup/dedup-keys.md (2,400 tokens)
Estimated size: 7,100 of 8,000 tokens
```

### 7.6 Checks

`mfd check` exits non-zero on errors so it can gate CI on the docs repo. The gate blocks merges only when the docs repo's branch rules require the check.

- Every node has valid frontmatter and a unique ID.
- Every link resolves: node IDs, `superseded_by`, and local Markdown links in node bodies.
- Superseded nodes name a replacement that exists.
- A Markdown file with a node `id` that isn't a node file, such as after a folder or tree rename, is an error.
- Every decision record has valid frontmatter, a node that exists, and a `supersedes` link that resolves.
- At most one accepted decision per `topic` within a node.
- With GitHub access, every referenced issue exists, and issues that name a node point to one that exists.
- Warnings: a tree with no root node, and one line for each nested tree it skipped.
- Optional: issue bodies that copy long passages of spec text instead of linking to the node.

### 7.7 Staleness

`mfd stale` compares each open linked issue's `spec-ref`, or its creation-time commit (7.2), with the latest commit that touched any file its node owns (7.1), images included. For each stale issue it prints the issue, the node and a diff summary of what changed. It only reports. A person or an agent decides whether to comment on the issue.

### 7.8 Writing to GitHub

The binary never writes to GitHub. Agents create issues, comments and pull requests with `gh`, guided by the plugin's skills. The harness's permission rules see each write, and a team can allow every `mfd` command without a prompt.

- **Planning work.** With the `manifestdocs-plan-issue` skill, the agent reads a node and drafts work items with acceptance criteria, using the repo's own issue templates. After the user approves, the agent creates the issues with `gh`, sets parents as sub-issues, and adds the top-level items to the node's `work` field.
- **Capturing a decision from an issue.** With the `manifestdocs-capture` skill, the agent reads the thread with `gh issue view`, drafts a decision record, ADR or spec change, and writes a record with `mfd decide`. A person opens the pull request.

### 7.9 Decision write-back

This is the shared-memory core of Manifest Docs. Decisions made during coding sessions become records in the docs repo that every engineer and agent on the project reads.

**Record format.** One file per decision, in the node's `decisions/` folder, named by date and topic. Separate files avoid merge conflicts when several sessions write at once.

```markdown
---
id: dc-3f1a8c2
node: fd-7c2e41
title: Default dedup window is ten minutes
status: accepted          # proposed | accepted | superseded | rejected
topic: dedup-window-default
decided_by: alastair      # optional
date: 2026-10-28
source: [example-org/eeb-dedup#212]
applies_to: [example-org/eeb-dedup:src/window.rs]
supersedes: null
---

**Decision.** The default window is ten minutes.
**Why.** Producers retry for up to eight minutes after a timeout.
**Rejected.** A five-minute window, which let late retries through as new events.
```

`decided_by` is optional, for teams whose records don't name people. `source` lists issues and pull requests, never harness session IDs. `applies_to` lists the code a decision affects, as `repo:path` text or links for readers, and the tool doesn't check it.

**What counts as significant.** The `manifestdocs-decide` skill gives agents a short test. Record a decision when the engineer:

- chooses between real alternatives
- fills a gap in a spec or departs from one
- sets a convention that others should follow
- rejects an approach and says why
- learns an outside constraint, such as an API limit or a vendor quirk

Skip personal preferences, which belong in the agent's local memory. Skip details that the code or the commit message already makes plain. Records go to a shared repo, so they never include secrets or personal data.

**Capture during a session.** When the engineer makes a call that passes the test, the agent drafts a record and asks the engineer to confirm, edit or drop it. On confirmation, the agent runs `mfd decide`. When a record names a decider, it's the engineer; the agent is never the decider.

**Capture at the end of a session.** A plugin hook prompts the agent once, before the session ends, to review the session and propose any decisions it missed. This catches decisions before the transcript is compacted or closed.

**Status.** `decide` sets the status from the repo's `decisions.merge` setting. Under `review`, it writes `accepted`, because the PR review is the approval. Under `proposed`, it writes `proposed`, and a later one-line PR accepts the record. Unattended runs always write `proposed`, and under `review` the person who opens their PR changes the records they endorse to `accepted` in that PR. A record gets the same status whether an agent or a person drafted it; `decided_by` and git history show who. Reviewers reject a record by setting `rejected` with the reason and merging it, not by closing the PR, so the record still stops the same idea from coming back.

**Input.** `mfd decide --file <path>` reads one record in the on-disk format, and `--file -` reads it from stdin. The tool fills in `id`, `date` and `status`, and rejects a supplied `id`, unknown fields and a status the repo setting forbids. It reports every validation error in one pass, each with the field, line and fix, and exits non-zero; `--json` returns the errors as an array. `--dry-run` prints the final record and its path without writing. Rerunning with the same node, topic and body exits 0 and prints the existing path, and a different body fails with a suggestion to use `supersedes`. `--unattended` marks factory runs, because agent shells often have no terminal to detect.

**Write path.** `decide` writes only the file and prints its path. It never branches or commits, and it warns when the docs clone it writes to is off its default branch or has uncommitted changes.

| Setting | Interactive session | Sandboxed factory run |
|---|---|---|
| Where `mfd decide` writes | The node's `decisions/` folder in a git worktree of the docs repo, on a new branch from the default branch. The `manifestdocs-decide` skill creates the worktree, so the main docs clone never switches branches | `.manifestdocs/decisions/` in the code repo's working directory, because the docs mount is read-only |
| What happens next | The skill commits, pushes and opens a PR with `gh`, then removes the worktree. With `merge: review` the team reviews the PR; with `merge: proposed` the engineer can merge it directly | The factory host moves the files into a docs branch, and a person opens the PR |
| Confirmation | The team reviews the PR, or a later review flips `proposed` to `accepted` | The same, after the run |

When the tool creates `.manifestdocs/`, it writes a `.gitignore` containing `*` inside it, so drafts never get committed to a code repo. `--out <dir>` overrides the folder.

The personal factory can use `merge: proposed` for speed. A team with governance needs, such as the EEB, uses `merge: review`.

**Read path.** Other sessions see a record once its PR merges and their docs clone fetches. `mfd context` includes accepted decisions for the node and its ancestors, newest first, inside the token budget. It also lists proposed decisions, marked as proposed. Superseded and rejected records are left out unless asked for. A rejected record still prevents the next session from proposing the same idea without a new reason.

**Promotion.** When a decision changes an approved spec or spans several nodes, the agent drafts an ADR from the record, in the docs repo's own ADR format. Once the ADR merges, the record links to it and its status becomes `superseded`.

### 7.10 Claude Code plugin

The plugin contains no logic. Each skill checks for the binary and calls it through the shell.

| Part | Contents |
|---|---|
| Skills | `manifestdocs-context` (run `context` before planning), `manifestdocs-decide` (spot, draft and confirm decisions, then open their PR from a worktree), `manifestdocs-author` (write a valid node), `manifestdocs-plan-issue` (draft linked issues and create them with `gh`), `manifestdocs-capture` (move a decision from an issue into docs) |
| Commands | `/manifestdocs:context`, `/manifestdocs:decide`, `/manifestdocs:stale`, `/manifestdocs:capture` |
| Hooks | Session start: inject a one-level tree outline. Before the session ends: prompt the agent once to review the session for unrecorded decisions. After edits in a docs repo: run `mfd check` |

Skills follow the open SKILL.md format so other harnesses can load them. An `AGENTS.md` snippet covers tools without skill support.

### 7.11 Sandbox use

- Tree commands read a local clone and need no network or token.
- In a software factory, the host runs `mfd context <issue>` and writes the output to the agent's context folder, such as `.context/`. That folder belongs to the factory: `mfd` writes to stdout, or to the path given with `--out`. The agent runs offline commands against a read-only mount of the docs repo.
- The run archive records the docs commit from the context header.
- Decision records from a sandboxed run go to `.manifestdocs/decisions/` (7.9). The host moves them into a docs branch after the run, so the container has no write access to the docs repo.

## 8. Architecture

### Language and crates

Rust, split into a library crate and a thin binary crate.

| Crate | Role |
|---|---|
| `manifestdocs-core` | Tree parsing, ID and link resolution, decision records, context assembly, budget trimming, checks |
| `manifestdocs` | CLI, GitHub reads, output formatting |

| Need | Choice |
|---|---|
| CLI parsing | `clap` |
| Tree walking that respects `.gitignore` | `ignore` |
| Reading the default branch without a checkout | `git symbolic-ref`, `git ls-tree` and one `git cat-file --batch` process, behind the same file-source interface as the working tree; `gix` later |
| Frontmatter and YAML | `serde` with `serde_yml` or `saphyr` |
| Markdown structure | `pulldown-cmark` |
| Git history and SHAs | `git` subprocess first, `gix` later |
| GitHub work tree | Read-only `gh api graphql` subprocess, batched per command |
| IDs | The operating system's random source, through `getrandom` |
| Token estimates | Characters divided by three, with 10% headroom; an exact tokenizer later behind a flag if needed |
| Errors | `thiserror` in core, `anyhow` in the binary |

Shelling out to `gh` reuses the user's auth, SSO and GitHub Enterprise hosts, with no token handling in Manifest Docs.

### Local folder

`.manifestdocs/` is the tool's only local folder (7.9). There's no cache or index: the tool reads the files on every run.

### Distribution

- The source repo and test repos are private, in the `dawson-design` GitHub org. Tests run against mock projects.
- Before 1.0, users build and install from a local clone with `cargo install --path`, and the Claude Code plugin installs from a local marketplace folder.
- At 1.0, `dist` (cargo-dist) builds releases for macOS ARM, Linux x86 and Linux ARM, with a Homebrew tap. A `gh-manifestdocs` repo publishes release assets with gh's naming convention, so `gh extension install` works and the tool runs as `gh manifestdocs`. The plugin installs from a marketplace repo.

### Future reuse

The core crate can power a `mfd mcp` subcommand for clients without a shell, or other tools that need to read the tree.

## 9. Prior art

| Tool | What it does | What Manifest Docs reuses from it |
|---|---|---|
| Beads | Git-backed issue tracker for agents with JSONL storage, a SQLite cache, hash IDs and `bd ready` | Logic in the binary instead of the prompt; focused queries; files as the source of truth; stable short IDs |
| Dex | Agent task tracker with description, context and result fields, three levels, GitHub and Shortcut sync, shipped as a skill | Plan-to-issues flow; CLI plus skill with no MCP; completion results as a source of decision records |

Both tools store work in the repo. Manifest Docs reads work from GitHub Issues and has no second tracker. Neither tool tracks how requirements change over time.

## 10. Success measures

| Measure | Target |
|---|---|
| Agent runs that start from `mfd context` in the personal factory | 90% within a month of release |
| Stale issues found before an agent starts work | Every stale issue flagged by `stale` before `agent-run` |
| Decisions recorded from sessions | Several a week in the personal factory; at least one a sprint per developer on the EEB pilot |
| Agent runs whose context included a decision made in an earlier session | Rising month over month |
| Proposed decisions reviewed within a week | 90% |
| `mfd check` in docs repo CI | Running on every docs PR for both factories |
| Time to build agent context | Under one second offline for a tree of 500 nodes |

## 11. Milestones

Manifest Docs dogfoods itself as early as each milestone allows. Its design docs, starting with this PRD, become its first tree at M0, and the rows in §12 become its first decision records at M2.

| Milestone | Scope | Done when |
|---|---|---|
| M0. Format | `manifestdocs.yaml`, node schema and IDs, ownership, user configuration, `init`, `tree`, `show`, `check` | The Manifest Docs design tree and the Root and Branch docs repo pass `check` |
| M1. Offline context | `context <id>`, linked docs, budget trimming, plugin with `manifestdocs-context` and `manifestdocs-author` | Sessions building Manifest Docs, and personal factory agents, start from `mfd context` |
| M2. Shared decisions | Decision record format, `decide`, `decisions`, decisions in `context`, reading the default branch from other repos, `manifestdocs-decide` skill, end-of-session hook, sandbox write path | A decision made in one Manifest Docs build session appears in another agent's next session once its PR merges |
| M3. Work tree | `context <issue>`, `work`, `stale`, `spec-ref` with the creation-time fallback, `manifestdocs-plan-issue` and `manifestdocs-capture` skills, the remaining commands | `agent-run` builds `.context/` from an issue in the docs repo, and one feature goes from node to linked issues to a merged decision record |
| M4. 1.0 | `dist` releases, Homebrew tap, `gh` extension, plugin marketplace repo | Installs on a fresh Mac and in a Linux factory container without a local build |
| Later | `mfd mcp` | Demand from a client without a shell |

## 12. Decisions

| Question | Decision |
|---|---|
| What does the tool cover? | Shared, living context on top of GitHub. Anything git, GitHub, `gh` or a Markdown link already does stays out (1, 4) |
| Which file marks a node? | A Markdown file named after its folder, with an `id` in its frontmatter, so editor tabs and search results name the feature. The root node is named after the tree (7.1) |
| Is the root node required? | No. Without one, `context` leaves out the project section and `check` warns (7.1) |
| What does a nested `manifestdocs.yaml` do? | It starts its own tree, which the outer tree skips (7.1) |
| Which files belong to a node? | Every file under its folder, except those under a deeper node or a nested tree. `show` lists them and `stale` watches them (7.1) |
| How are node folders named? | Kebab case by default, so links need no escaped spaces. `check` accepts existing names (7.1) |
| What do IDs look like? | `fd-` plus 6 random hex characters for nodes, `dc-` plus 7 for records, generated again on collision. `check` enforces uniqueness in the tree; reuse after deletion isn't tracked (7.1) |
| Which issue hierarchy does the tool support? | GitHub sub-issues at any depth, or flat issues (7.3) |
| Where are issue-to-node links stored? | Top-level issues in the node's `work` field, inherited by sub-issues; a `feature:` body line for issues outside that hierarchy. Not in labels or Projects fields (7.2) |
| Does `mfd` write to GitHub? | No. Agents create issues, comments and PRs with `gh`, so the harness's permission rules see each write (7.8) |
| Where do planned and shipped dates live? | In GitHub Projects. `mfd` shows only each issue's open or closed state (4) |
| How do images, style guides and external references work? | As ordinary Markdown links (7.1) |
| Which version of the docs does the tool read? | The working tree inside the docs clone, with a warning when it's off the default branch. From any other repo, the default branch from `origin/HEAD`, never fetched, with its commit and date in the context header (7.1) |
| Which links does `context` follow? | Local Markdown links in the target node's body, one hop. Nodes have no `docs` field (7.5) |
| How accurate must token budgets be? | Approximate: characters divided by three with 10% headroom, with an exact tokenizer as a later option (7.5) |
| What happens without `--budget`? | Nothing is trimmed. The output ends with its estimated size, and the plugin always passes a budget (7.5) |
| Where do decisions from coding sessions go? | One record per decision in the node's `decisions/` folder, written by `mfd decide` and shared through the docs repo (7.9) |
| Who confirms a decision? | The engineer in the session. Unattended runs write `proposed` records for later review (7.9) |
| Which status does `decide` write? | `accepted` under `merge: review`, `proposed` under `merge: proposed` and in unattended runs. Reviewers reject by merging a `rejected` record (7.9) |
| Does `decide` touch git? | No. It writes the file; the skill branches in a worktree, commits and opens the PR (7.9) |
| What does `decide` read? | One record in the on-disk format, from `--file` or stdin. It fills in `id`, `date` and `status` (7.9) |
| Does `mfd` call a model? | No. Agents judge what is significant; the binary stores, validates and retrieves (6) |
| Does the tool need GitHub to be useful? | No. Tree commands work with only `manifestdocs.yaml`; work commands turn on with a `github` block (7.1) |
| Which local folder does the tool use? | `.manifestdocs/`, ignored by git. A factory's `.context/` belongs to the factory (7.11) |
| Where is the tool built and tested? | Private repos in `dawson-design`, tested against mock projects, installed from a local build until 1.0 (8) |

## 13. Open questions

- How should the agent choose a node when a session decision applies to none, or to several?
- Should a rejected decision have an expiry date, so a team can revisit it without a new reason after some time?
