---
id: dc-5283b0d
node: fd-3e9d0a
title: "Quantities stay metric"
status: accepted
topic: metric-only
decided_by: priya
date: 2026-09-20
---

**Decision.** Recipes store and show grams, millilitres, teaspoons, tablespoons and whole items. The cookbook doesn't convert to cups or ounces.

**Why.** Every recipe in the catalogue was tested in metric, and conversions for dry ingredients depend on density.

**Rejected.** A `--us` flag for cups and ounces, until readers outside the UK ask for it.
