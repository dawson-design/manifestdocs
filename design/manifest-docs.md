---
id: fd-622eed
title: Manifest Docs
status: draft
summary: A CLI that adds shared, living context on top of GitHub, with a feature tree in a docs repo, focused agent context and shared decision records.
---

# Manifest Docs (`mfd`)

**Owner:** Alastair Dawson · **Domain:** manifestdocs.ai

Manifest Docs is a command-line tool that adds shared, living context on top of GitHub and GitHub Enterprise. It turns a folder of Markdown in a docs repo into a feature tree that agents and people can walk, and its files read as ordinary pages on GitHub without the tool. Each node is a folder with the requirements and decisions for one feature. The tool links nodes to work in GitHub Issues and flags issues whose requirements changed after planning. It also assembles focused context for coding agents and records the decisions engineers make during coding sessions, so every person and agent on the project shares them.

It ships as one Rust binary (`manifestdocs`, alias `mfd`) and a thin Claude Code plugin. Git stores the tree and its history. GitHub hosts and reviews it. The binary reads from GitHub and never writes to it, and anything that git, GitHub, `gh` or a Markdown link already does stays out of the tool. There is no server, database or account.

The [product brief](product.md) covers the problem, users, concepts, prior art and success measures. [Architecture](architecture.md) covers the language, crates and local folder, and [milestones](milestones.md) set the build order.

## Goals

1. Give agents the right context for an issue in one command, within a token budget.
2. Keep requirements and decisions in versioned docs, and keep work in GitHub Issues.
3. Show when an issue's requirements changed after it was planned.
4. Turn significant decisions from coding sessions, issue threads and agent results into shared, reviewed records in the docs repo.
5. Work offline inside a sandbox with no credentials.
6. Run in any agent harness that has a shell.
7. Keep the docs tree readable on GitHub, as ordinary pages, for people who don't have the tool.

## Non-goals

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

## Rules the tool supports

- Issues point to docs and never restate them.
- A decision is real once it merges into the docs repo. Comments and sessions can discuss it.
- People make decisions. Agents notice and draft them, and the engineer confirms.
- The binary never calls a model. Agents do the judgment; Manifest Docs stores, validates and retrieves.
- The binary reads from GitHub and never writes to it. Agents write with `gh`.
- Each link is recorded in one place. A feature node lists its top-level work items, and their sub-issues inherit the link.
- An issue can record the docs commit it was planned against. Without one, the tool uses the docs commit from when the issue was created.
- Images, diagrams, style guides and external references are Markdown links, not features.

## Features

- [Tree format](tree-format/tree-format.md)
- [CLI](cli/cli.md)
- [Agent context](context/context.md)
- [Checks](checks/checks.md)
- [Links to work](work-links/work-links.md)
- [Decision records](decision-records/decision-records.md)
- [Claude Code plugin](plugin/plugin.md)
- [Sandbox use](sandbox/sandbox.md)
- [Distribution](distribution/distribution.md)
