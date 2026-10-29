---
id: fd-050937
title: "Recipe page"
status: approved
summary: "Shows one recipe's header and ingredients with `cookbook show`."
---

# Recipe page

`cookbook show <slug>` prints one recipe for a reader to cook from.

## Requirements

### Requirement: Header

The page SHALL start with the recipe's title on its own line, then `Serves <n> · <minutes> min`, or `Serves <n> · time unknown` when the total time is unknown, then a blank line.

#### Scenario: Known time

- WHEN a reader runs `cookbook show shakshuka`
- THEN the first two lines are `Shakshuka` and `Serves 2 · 35 min`

### Requirement: Ingredients

The page SHALL list each ingredient on its own line as `- <quantity> <unit> <name>`, in the recipe's order. Whole items have no unit, as in `- 4 eggs`. Quantities drop trailing zeros and show at most two decimal places.

#### Scenario: Grams and whole items

- WHEN a reader runs `cookbook show shakshuka`
- THEN the ingredients include `- 400 g tomatoes` and `- 4 eggs`

### Requirement: Unknown recipe

A slug that names no recipe SHALL print an error naming the slug to stderr and exit with 2.
