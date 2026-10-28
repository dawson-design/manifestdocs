# Manifest Docs: product brief

## Problem

Teams keep two kinds of knowledge in different places, and agents need both.

- **Decisions live in docs.** PRDs, specs, design docs and ADRs sit in a separate docs repo. They change through reviewed commits, so their history is clear.
- **Work lives in issues.** Issues, often grouped under parents such as epics and initiatives, track who does what and when. No single issue, even a top-level one, can hold all the product and design decisions behind it. Edits to issue bodies and decisions made in comments leave no reviewable history.

Agents fall into the gap between the two. An issue gives them the task without the reasoning. The docs repo gives them the reasoning without a way to find the part that matters. When a spec changes after someone plans an issue, nothing tells the agent or the reviewer. Decisions made in issue threads never reach the docs, so the next agent repeats the debate.

Coding sessions add a third gap. An engineer makes real decisions while pairing with an agent: which approach to take, which convention to follow, which idea failed and why. Claude often saves these as memories, but that memory is local to one user. The next engineer, and every other agent, starts without it. The decision exists only in a transcript nobody will read again.

The earlier version of Manifest tried to solve this with a hosted platform: an MCP server, a web app, a database and auth. It grew too large. The core ideas, context as a feature tree and decisions written back as shared memory, work without any of that.

## Users

| User | Need |
|---|---|
| Coding agent | Read the requirements, decisions and work hierarchy for its issue before planning |
| Solo developer | Keep a personal software factory's agents aligned with game rules and design docs |
| Engineering team | Keep specs, ADRs and stories consistent across several code repos and a shared docs repo |
| Factory tooling | Build `.context/` for each agent run, gate docs PRs in CI, report stale issues |
| Coding agent in a session | Notice when the engineer makes a significant decision and propose a record of it |
| Docs-lane agent | Draft ADRs and spec updates from decisions captured in issues and PR results |

## Concepts

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

## Prior art

| Tool | What it does | What Manifest Docs reuses from it |
|---|---|---|
| Beads | Git-backed issue tracker for agents with JSONL storage, a SQLite cache, hash IDs and `bd ready` | Logic in the binary instead of the prompt; focused queries; files as the source of truth; stable short IDs |
| Dex | Agent task tracker with description, context and result fields, three levels, GitHub and Shortcut sync, shipped as a skill | Plan-to-issues flow; CLI plus skill with no MCP; completion results as a source of decision records |

Both tools store work in the repo. Manifest Docs reads work from GitHub Issues and has no second tracker. Neither tool tracks how requirements change over time.

## Success measures

| Measure | Target |
|---|---|
| Agent runs that start from `mfd context` in the personal factory | 90% within a month of release |
| Stale issues found before an agent starts work | Every stale issue flagged by `stale` before `agent-run` |
| Decisions recorded from sessions | Several a week in the personal factory; at least one a sprint per developer on the EEB pilot |
| Agent runs whose context included a decision made in an earlier session | Rising month over month |
| Proposed decisions reviewed within a week | 90% |
| `mfd check` in docs repo CI | Running on every docs PR for both factories |
| Time to build agent context | Under one second offline for a tree of 500 nodes |
