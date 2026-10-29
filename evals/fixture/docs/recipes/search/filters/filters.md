---
id: fd-3e8169
title: "Search filters"
status: approved
summary: "Flags on `cookbook search` that narrow the results, and the rules every filter follows."
---

# Search filters

Filters narrow search results. Each filter is a flag on `cookbook search`, and filters combine: a recipe must pass every filter the reader gives.

## Rules for every filter

- A filter never hides a recipe because the recipe is missing the data the filter reads. A recipe with no total time, no cuisine, or an ingredient that isn't in `data/ingredients.json` stays in the results of a filter on that data. Most of these are recipes typed in from family recipe cards, and hiding them made them impossible to find.
- Filters run after text matching and keep the order of the results.
- A filter value that matches nothing, such as an unknown cuisine, returns `No recipes found.` rather than an error.

## Features

- [Cuisine](cuisine/cuisine.md)
- [Dietary](dietary/dietary.md)
- [Cooking time](cooking-time/cooking-time.md)
