---
id: fd-2b6f10
title: "Recipes"
status: approved
summary: "The recipe catalogue: listing, the recipe page, search, scaling and ratings."
---

# Recipes

The catalogue is a curated set of recipes in `data/recipes.json`. Each recipe has a slug, a title, a cuisine, the number of people it serves, its total time in minutes and its ingredients. The total time is `null` when we don't know it, as for recipes typed in from family recipe cards.

Each ingredient has a name, a quantity and a unit. The name matches an entry in `data/ingredients.json`.

## Requirements

### Requirement: List recipes

`cookbook list` SHALL print one line per recipe, in catalogue order: its slug, two spaces, then its title.

#### Scenario: Browse the catalogue

- WHEN a reader runs `cookbook list`
- THEN one line reads `shakshuka  Shakshuka`

## Features

- [Recipe page](recipe-page/recipe-page.md)
- [Search](search/search.md)
- [Scaling](scaling/scaling.md)
- [Ratings](ratings/ratings.md)
- [Import from recipe cards](import/import.md)
