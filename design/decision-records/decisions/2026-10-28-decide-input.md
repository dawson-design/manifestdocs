---
id: dc-6ae4e50
node: fd-ae6745
title: "decide reads a whole record from a file or stdin"
status: accepted
topic: decide-input
decided_by: alastair
date: 2026-10-28
---

**Decision.** `mfd decide --file <path>`, or `--file -` for stdin, reads one record in the on-disk format. It fills in `id`, `date` and `status`, rejects a supplied `id` and unknown fields, reports every error in one pass, and supports `--dry-run`, `--json` and `--unattended`. A rerun with the same body exits 0.

**Why.** One format then serves `decide`, `check` and reviewers, and the body needs no JSON escaping; Aider measured worse output when models write code inside JSON. clig.dev recommends `-` for stdin, `--dry-run`, and no prompts without a terminal.

**Rejected.** A flag per field. JSON on stdin. No `decide` command at all, with agents writing files as changesets does, which leaves ID generation and the status rules to each agent.
