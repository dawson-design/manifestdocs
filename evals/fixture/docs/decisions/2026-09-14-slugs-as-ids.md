---
id: dc-78c4dc2
node: fd-3e9d0a
title: "Recipe slugs are the only recipe IDs"
status: accepted
topic: slugs-as-ids
decided_by: sam
date: 2026-09-14
---

**Decision.** A recipe's slug, such as `red-lentil-dal`, identifies it in commands, data files and links. Slugs never change once a recipe ships.

**Why.** Readers type slugs, and the ratings export from the website is keyed by them.

**Rejected.** Numeric IDs, which readers can't remember, and titles, which editors change.
