---
id: fd-6ee960
title: "Weekly plan"
status: draft
summary: "Adds recipes to days and meals in the current week's plan."
---

# Weekly plan

## Requirements

### Requirement: Add a meal

`cookbook plan add <day> <meal> <slug>` SHALL put a recipe in the plan, replacing any recipe already in that slot.

#### Scenario: Tuesday dinner

- WHEN a reader runs `cookbook plan add tue dinner red-lentil-dal`
- THEN `cookbook plan` shows Red lentil dal for Tuesday dinner

### Requirement: Show the plan

`cookbook plan` SHALL print the week, one line per planned meal, as `<day> <meal>: <title>`.
