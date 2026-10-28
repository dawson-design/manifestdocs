---
id: dc-b62fc0c
node: fd-630023
title: "Only .manifestdocs/decisions ignores itself"
status: accepted
topic: local-folder
decided_by: alastair
date: 2026-10-28
supersedes: dc-e64c696
---

**Decision.** `.manifestdocs/` holds the committed tree pointer, `config.yaml`, and the sandbox's decision drafts in `decisions/`. The tool writes a `.gitignore` containing `*` inside `decisions/` when it creates that folder, so drafts are never committed. `mfd` writes context to stdout or `--out`, and a factory's `.context/` belongs to the factory.

**Why.** A single repo commits its pointer, so the whole folder can no longer ignore itself. The name and the self-ignoring pattern carry over from the decision this replaces.

**Rejected.** A second dotfolder for the pointer.
