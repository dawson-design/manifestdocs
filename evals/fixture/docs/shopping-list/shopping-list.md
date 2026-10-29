---
id: fd-b9b186
title: "Shopping list"
status: approved
summary: "Combines the ingredients of the recipes a reader picks into one list with `cookbook shop`."
---

# Shopping list

Readers pick the recipes they plan to cook this week and get one list to take to the shop.

## Requirements

### Requirement: One list for several recipes

`cookbook shop <slug>...` SHALL print the ingredients of every named recipe, one per line, in the format the [recipe page](../recipes/recipe-page/recipe-page.md) uses: `- <quantity> <unit> <name>`, with no unit for whole items.

#### Scenario: One recipe

- WHEN a reader runs `cookbook shop greek-salad`
- THEN the list has five lines, including `- 300 g tomatoes` and `- 1 cucumber`

### Requirement: Combine the same ingredient

An ingredient that appears in more than one recipe with the same unit SHALL appear once, with its quantities added. The same ingredient in different units stays on separate lines.

#### Scenario: Eggs for two recipes

- WHEN a reader runs `cookbook shop shakshuka pancakes`
- THEN the list has one line for eggs, `- 6 eggs`

### Requirement: Unknown recipe

A slug that names no recipe SHALL print an error naming the slug to stderr, print no list, and exit with 2.
