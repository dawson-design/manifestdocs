---
id: fd-30f76d
title: "Scaling"
status: approved
summary: "Shows a recipe for a different number of people with `cookbook show --servings`."
---

# Scaling

Readers cooking for more or fewer people than a recipe serves scale it to the number they need.

## Requirements

### Requirement: Scale to a number of servings

`cookbook show <slug> --servings N` SHALL show the recipe for N servings. Every quantity multiplies by N divided by the servings the recipe states, and the header reads `Serves N`.

#### Scenario: Double a recipe

- WHEN a reader runs `cookbook show shakshuka --servings 4`
- THEN the header reads `Serves 4 · 35 min`
- AND the ingredients include `- 800 g tomatoes` and `- 8 eggs`

### Requirement: Servings limit

N SHALL be a whole number from 1 to 24. Any other value SHALL print an error and exit with 2.

#### Scenario: Too many

- WHEN a reader runs `cookbook show pancakes --servings 30`
- THEN the command prints an error and exits with 2

## Background

We ran two rounds of testing with home cooks before building this. The [scaling user-testing notes](user-testing.md) record what they did and what they asked for.
