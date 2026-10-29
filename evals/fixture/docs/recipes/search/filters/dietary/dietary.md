---
id: fd-4218a0
title: "Dietary filters"
status: approved
summary: "Limits search to recipes that suit a diet, using the tags in `data/ingredients.json`."
---

# Dietary filters

Dietary filters read the `tags` of each ingredient in `data/ingredients.json`.

## Requirements

### Requirement: Vegetarian

`cookbook search --vegetarian` SHALL leave out recipes with any ingredient tagged `meat` or `fish`.

#### Scenario: Stew is left out

- WHEN a reader searches with `--vegetarian`
- THEN Grandma's beef stew, Fish tacos and Chicken tikka masala are not in the results
- AND Shakshuka is

## Later

A `--vegan` flag, once every ingredient that comes from an animal has a tag. Honey and parmesan made with animal rennet are the open cases.
