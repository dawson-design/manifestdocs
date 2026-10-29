# H5: control, everything in the target node

**Hidden fact.** None outside the target. `docs/recipes/ratings/ratings.md` states the line format, `★ 4.3 (3)`, and that a recipe with fewer than 3 ratings shows `(not enough ratings)`. The prompt mentions neither. Both arms read the node the prompt names, so they should tie.

**Obvious implementation without the node.** An average for every rated recipe, including Pancakes' two ratings, in a format of the agent's choosing.

| Id | Kind | Pass when |
|---|---|---|
| `tests_pass` | base | `python3 -m unittest` passes, including the agent's own tests |
| `shows_average` | base | `cookbook list` prints `shakshuka  Shakshuka  ★ 4.3 (3)` |
| `counts_ratings` | base | It prints `pasta-pomodoro  Pasta al pomodoro  ★ 4.3 (12)` |
| `rounds_to_one_decimal` | base | It prints `chicken-tikka-masala  Chicken tikka masala  ★ 4.7 (7)` |
| `too_few_ratings` | hidden | It prints `pancakes  Pancakes  (not enough ratings)` (two ratings) |
| `no_ratings` | hidden | It prints `baba-ganoush  Baba ganoush  (not enough ratings)` (no ratings) |
