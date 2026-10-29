---
id: fd-91ab5d
title: "Ranking"
status: draft
summary: "Orders search results by how well each recipe matches, once the rules are agreed."
---

# Ranking

Search returns results in catalogue order today. Ranking will put the best matches first, following the [ranking rules](ranking-rules.md) once they're agreed.

## Requirements

### Requirement: Title matches first

Search SHALL list recipes that match the query in their title before recipes that match only in their ingredients.

#### Scenario: Eggs in the title

- WHEN a reader searches for `eggs`
- THEN Soft-boiled eggs with soldiers comes before Pancakes
