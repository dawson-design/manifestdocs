---
id: dc-4c67849
node: fd-7c2e41
title: "Search matches whole words, not parts of words"
status: accepted
topic: whole-words
decided_by: priya
date: 2026-09-11
source: [example-org/cookbook#17]
---

**Decision.** A query word matches a whole word in a recipe's title or ingredient names, never part of a word.

**Why.** Substring matching returned Baba ganoush for "egg", through "eggplant", and Soft-boiled eggs for "oil", through "boiled", and readers stopped trusting the results.

**Rejected.** Substring and prefix matching, for those false matches. Fuzzy matching with `difflib`, which matched "lime" to "lemon".
