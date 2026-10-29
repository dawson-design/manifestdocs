# H3: binding decision in another node's `decisions/`

**Hidden fact.** `docs/decisions/2026-10-06-aisle-order.md` (dc-9c7bffc), on the root node, says every list a reader takes to the shop groups ingredients by their `aisle` in `data/ingredients.json`, in the order a reader walks the store: produce, bakery, meat-fish, dairy-eggs, then pantry. It applies to the shopping list and the pantry check, so the nearest-common-ancestor rule put it on the root. The target node and its own `decisions/` don't mention aisles.

**Obvious implementation.** Recipe order or alphabetical order. Sorting by aisle name alphabetically also fails, because it puts produce last.

| Id | Kind | Pass when |
|---|---|---|
| `tests_pass` | base | `python3 -m unittest` passes, including the agent's own tests |
| `lists_each_once` | base | `cookbook shop shakshuka pancakes soft-boiled-eggs` prints each of the 11 ingredients on exactly one `- ` line |
| `merges_eggs` | base | That list has the line `- 8 eggs` |
| `merges_butter` | base | That list has the line `- 40 g butter` |
| `unknown_exits_2` | base | `cookbook shop no-such-recipe` exits with 2 |
| `aisle_order` | hidden | That list's ingredient lines run produce, bakery, meat-fish, dairy-eggs, pantry; aisle headings are allowed |
