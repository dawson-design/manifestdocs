# H4: deciding detail in a linked supporting doc

**Hidden fact.** `docs/recipes/scaling/user-testing.md`, linked from the target node's Background section as "scaling user-testing notes", says scaled quantities round: whole items up to the next whole number, teaspoons and tablespoons to the nearest quarter, grams and millilitres to the nearest whole number. The rule sits in the middle of the doc, under "Quantities must be measurable". The node itself says nothing about rounding.

**Obvious implementation.** Multiply and print with the existing two-decimal formatting, giving `1.5 eggs`, `22.5 g butter` and `0.67 tsp mustard`.

| Id | Kind | Pass when |
|---|---|---|
| `tests_pass` | base | `python3 -m unittest` passes, including the agent's own tests |
| `header_shows_servings` | base | `cookbook show shakshuka --servings 4` prints `Serves 4 · 35 min` |
| `doubles_grams` | base | It prints `- 800 g tomatoes` |
| `doubles_eggs` | base | It prints `- 8 eggs` |
| `over_24_exits_2` | base | `cookbook show pancakes --servings 30` exits with 2 |
| `zero_exits_2` | base | `cookbook show pancakes --servings 0` exits with 2 |
| `eggs_round_up` | hidden | `cookbook show pancakes --servings 3` prints `- 2 eggs` (1.5 rounded up) |
| `onion_rounds_up` | hidden | `cookbook show shakshuka --servings 3` prints `- 2 onion` (1.5 rounded up) |
| `grams_are_whole` | hidden | `cookbook show pancakes --servings 3` prints `- 22 g butter` or `- 23 g butter`, not 22.5 |
| `spoons_to_quarter` | hidden | `cookbook show potato-salad --servings 4` prints mustard as `0.75`, `¾` or `3/4` tsp, not 0.67 |
| `spoons_to_quarter_above_one` | hidden | The same prints mayonnaise as `2.75`, `2 ¾` or `2 3/4` tbsp, not 2.67 |
