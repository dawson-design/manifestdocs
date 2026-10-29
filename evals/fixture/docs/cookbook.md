---
id: fd-3e9d0a
title: "Cookbook"
status: approved
summary: "A command-line cookbook that finds, scales and shops for home recipes, offline."
---

# Cookbook

Cookbook is a command-line cookbook for home cooks. Readers browse and search a curated set of recipes, scale a recipe for the people they're feeding, and turn the recipes they pick into a list for the shop.

## Rules for every feature

- The cookbook runs offline. It reads recipes, ingredients and ratings from the JSON files in `data/`, and never calls a network service.
- Quantities are metric: grams, millilitres, teaspoons, tablespoons or whole items. The cookbook never converts to cups or ounces.
- A recipe's slug is its ID everywhere: in commands, data files and links.
- Output is plain text, one item per line, so readers can pipe it into other tools. Errors go to stderr, and a command that can't run exits with 2.

## Data

| File | Holds |
|---|---|
| `data/recipes.json` | Every recipe: slug, title, cuisine, servings, total time and ingredients |
| `data/ingredients.json` | Every ingredient by name: its store aisle and dietary tags |
| `data/ratings.json` | The ratings export from the website, by recipe slug |

## Features

- [Recipes](recipes/recipes.md)
- [Shopping list](shopping-list/shopping-list.md)
- [Pantry](pantry/pantry.md)
- [Meal plan](meal-plan/meal-plan.md)
