---
id: fd-66ed91
title: "Meal plan"
status: draft
summary: "Plans a week of meals and feeds the shopping list."
---

# Meal plan

Readers plan the week's meals, then shop for the whole plan at once.

## Requirements

### Requirement: A plan is a week

A meal plan SHALL cover seven days, from Monday, with up to one recipe per meal: breakfast, lunch and dinner.

### Requirement: Shop for the plan

`cookbook shop --plan` SHALL list the ingredients for every recipe in the current plan, as `cookbook shop` lists them for named recipes.

## Features

- [Weekly plan](weekly-plan/weekly-plan.md)
- [Leftovers](leftovers/leftovers.md)
