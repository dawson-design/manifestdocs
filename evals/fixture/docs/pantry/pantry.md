---
id: fd-40fe67
title: "Pantry"
status: draft
summary: "Lets readers record what they have at home, and checks a shopping list against it."
---

# Pantry

Readers keep a list of what they have at home, so the shopping list can leave it out.

## Requirements

### Requirement: Record the pantry

`cookbook pantry add <ingredient> <quantity> <unit>` SHALL record an ingredient the reader has, in a file in the reader's home folder.

### Requirement: Pantry check

`cookbook pantry check <slug>...` SHALL print the ingredients the named recipes need that the pantry doesn't cover, in the shopping list's format.

#### Scenario: Eggs at home

- WHEN the pantry has 6 eggs
- AND a reader runs `cookbook pantry check shakshuka pancakes`
- THEN the check has no line for eggs

## Open questions

- Does the pantry expire items, such as milk after a week?
- Should `cookbook shop` subtract the pantry by default once this ships?
