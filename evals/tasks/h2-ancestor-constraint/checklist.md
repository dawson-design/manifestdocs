# H2: constraint stated on an ancestor node

**Hidden fact.** The parent node, `docs/recipes/search/filters/filters.md`, says a filter never hides a recipe because the recipe is missing the data the filter reads. The target node, `cooking-time.md`, doesn't repeat it. Two recipes, Grandma's beef stew and Aunt Rose's banana bread, have `"minutes": null`.

**Obvious implementation.** `r.minutes is not None and r.minutes <= n`, which drops recipes with an unknown time.

| Id | Kind | Pass when |
|---|---|---|
| `tests_pass` | base | `python3 -m unittest` passes, including the agent's own tests |
| `keeps_pancakes` | base | `cookbook search --max-minutes 30` lists `pancakes` (25 min) |
| `keeps_tacos_at_limit` | base | `cookbook search --max-minutes 30` lists `fish-tacos` (30 min) |
| `drops_shakshuka` | base | `cookbook search --max-minutes 30` doesn't list `shakshuka` (35 min) |
| `works_with_query` | base | `cookbook search eggs --max-minutes 15` lists `soft-boiled-eggs` |
| `zero_exits_2` | base | `cookbook search --max-minutes 0` exits with 2 |
| `keeps_unknown_stew` | hidden | `cookbook search --max-minutes 30` lists `grandmas-beef-stew` (unknown time) |
| `keeps_unknown_bread` | hidden | `cookbook search --max-minutes 30` lists `banana-bread` (unknown time) |
