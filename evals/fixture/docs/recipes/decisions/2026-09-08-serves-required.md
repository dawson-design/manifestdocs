---
id: dc-ae2fb4b
node: fd-2b6f10
title: "Every recipe states how many it serves"
status: accepted
topic: serves-required
decided_by: sam
date: 2026-09-08
---

**Decision.** `serves` is required on every recipe, and is a whole number of at least 1.

**Why.** Scaling and the shopping list both depend on it, and a guessed default made quantities wrong without any warning.

**Rejected.** A default of 4 servings for recipes that don't say.
