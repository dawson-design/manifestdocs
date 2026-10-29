---
id: fd-66775f
title: "Cooking time filter"
status: approved
summary: "Limits search to recipes that fit the time a reader has, with `--max-minutes`."
---

# Cooking time filter

Readers in a hurry limit search to recipes they can cook in the time they have. The filter reads each recipe's total time, `minutes` in `data/recipes.json`.

## Requirements

### Requirement: Maximum total time

`cookbook search --max-minutes N` SHALL keep recipes whose total time is N minutes or less.

#### Scenario: Quick dinners

- WHEN a reader searches with `--max-minutes 30`
- THEN Pancakes, at 25 minutes, and Fish tacos, at 30 minutes, are in the results
- AND Shakshuka, at 35 minutes, is not

#### Scenario: With a query

- WHEN a reader searches for `eggs` with `--max-minutes 15`
- THEN Soft-boiled eggs with soldiers is in the results

### Requirement: A valid limit

N SHALL be a whole number of at least 1. Any other value SHALL print an error and exit with 2.
