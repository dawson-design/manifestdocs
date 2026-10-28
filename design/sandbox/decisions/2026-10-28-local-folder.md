---
id: dc-e64c696
node: fd-630023
title: "The tool's local folder is .manifestdocs"
status: superseded
topic: local-folder
decided_by: alastair
date: 2026-10-28
---

**Decision.** Sandboxed runs write decision drafts to `.manifestdocs/decisions/`, and the tool writes a `.gitignore` containing `*` inside the folder when it creates it. `mfd` writes context to stdout or `--out`, and a factory's `.context/` belongs to the factory.

**Why.** `.context/` is already used by factory tooling and by other tools such as ctxgen. `.manifest` reads as build or packaging metadata in web, Android, Unity and Cargo projects. `.manifestdocs` matches the binary and config names. The self-ignoring folder follows pytest's `.pytest_cache`.

**Rejected.** `.context/` and `.manifest/`.
