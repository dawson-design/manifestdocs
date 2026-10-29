---
id: dc-9c7bffc
node: fd-3e9d0a
title: "Lists for the shop follow the store's aisle order"
status: accepted
topic: aisle-order
decided_by: priya
date: 2026-10-06
source: [example-org/cookbook#48]
---

**Decision.** Any list a reader takes to the shop, such as the shopping list and the pantry check, groups ingredients by their `aisle` in `data/ingredients.json` and puts the aisles in the order a reader walks the store: produce, bakery, meat-fish, dairy-eggs, then pantry. Within an aisle, ingredients keep the order they were first added.

**Why.** In the paper prototype, readers with an alphabetical list crossed the store four or five times per trip.

**Rejected.** Alphabetical order and recipe order, which both sent readers back and forth between aisles. Aisle names in alphabetical order, which puts produce last.
