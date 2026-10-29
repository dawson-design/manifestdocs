---
id: fd-7c2e41
title: "Recipe search"
status: approved
summary: "Finds recipes by words in their title or ingredients, then narrows them with filters."
---

# Recipe search

`cookbook search [QUERY]` finds recipes by words in their title or ingredient names, then applies any [filters](filters/filters.md). It prints each match as `cookbook list` does, in catalogue order, or `No recipes found.` when nothing matches.

## Requirements

### Requirement: Match every word

Search SHALL return the recipes that match every word of the query in their title or an ingredient name, ignoring case.

#### Scenario: Two ingredients

- WHEN a reader searches for `olive oil feta`
- THEN the results are Shakshuka and Greek salad

#### Scenario: A title word

- WHEN a reader searches for `shakshuka`
- THEN the only result is Shakshuka

### Requirement: Empty query

Search with no query SHALL return every recipe that passes the filters.

#### Scenario: Filters alone

- WHEN a reader runs `cookbook search --cuisine indian`
- THEN the results are Chicken tikka masala and Red lentil dal

## Features

- [Filters](filters/filters.md)
- [Ranking](ranking/ranking.md)
