---
id: dc-703b34e
node: fd-3e9d0a
title: "Python standard library only"
status: accepted
topic: stdlib-only
decided_by: sam
date: 2026-09-02
---

**Decision.** The cookbook uses Python 3.9 and its standard library, with no third-party packages.

**Why.** Readers run it from a copy of the repo on whatever Python their machine has, with no install step.

**Rejected.** Click for the command line, which needs pip. A packaged release, which we can revisit once the feature set settles.
