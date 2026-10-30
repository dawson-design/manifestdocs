---
id: fd-630023
title: "Sandbox use"
status: draft
summary: "How a software factory builds context on the host and collects decision records from sandboxed runs."
---

# Sandbox use

- Tree commands read a local clone and need no network or token.
- In a software factory, the host runs `mfd context <issue>` and writes the output to the agent's context folder, such as `.context/`. That folder belongs to the factory: `mfd` writes to stdout, or to the path given with `--out`. The agent runs offline commands against a read-only mount of the docs repo. When another user owns that mount, git refuses it until the container's git config trusts it, such as with `git config --system --add safe.directory /docs`; the tool keeps that check rather than overriding it.
- The run archive records the docs commit from the context header.
- Decision records from a sandboxed run go to `.manifestdocs/decisions/` (see the [write path](../decision-records/decision-records.md#write-path)). The host moves them into a docs branch after the run, so the container has no write access to the docs repo.
