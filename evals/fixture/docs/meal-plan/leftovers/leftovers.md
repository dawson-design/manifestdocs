---
id: fd-ef77fd
title: "Leftovers"
status: draft
summary: "Marks meals that use up an earlier recipe's leftovers, so the plan doesn't shop for them twice."
---

# Leftovers

Readers who cook a recipe for more people than they feed eat the rest later in the week.

## Requirements

### Requirement: Plan leftovers

`cookbook plan add <day> <meal> leftovers:<slug>` SHALL mark a meal as leftovers of a recipe planned earlier in the week, and the plan's shopping list SHALL NOT add that recipe's ingredients again.

## Open questions

- Should the plan warn when leftovers come more than three days after the recipe?
