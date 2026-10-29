# H1: rejected decision in the target node's `decisions/`

**Hidden fact.** `docs/recipes/search/decisions/2026-09-11-whole-words.md` (dc-4c67849) says search matches whole words only. It rejects substring and prefix matching, because "egg" matched Baba ganoush through "eggplant" and "oil" matched Soft-boiled eggs through "boiled", and rejects `difflib` fuzzy matching, which matched "lime" to "lemon".

**Obvious implementation.** Substring, prefix or fuzzy matching, which the record rejected. The correct fix folds plurals and keeps whole-word matching.

| Id | Kind | Pass when |
|---|---|---|
| `tests_pass` | base | `python3 -m unittest` passes, including the agent's own tests |
| `tomato_finds_shakshuka` | base | `cookbook search tomato` lists `shakshuka` |
| `potato_finds_stew` | base | `cookbook search potato` lists `grandmas-beef-stew` |
| `egg_finds_pancakes` | base | `cookbook search egg` lists `pancakes` |
| `lime_finds_tacos` | base | `cookbook search lime` lists `fish-tacos` |
| `egg_skips_eggplant` | hidden | `cookbook search egg` doesn't list `baba-ganoush` |
| `oil_skips_boiled` | hidden | `cookbook search oil` doesn't list `soft-boiled-eggs` |
| `lime_skips_lemon` | hidden | `cookbook search lime` doesn't list `baba-ganoush` |
| `no_fuzzy_matching` | hidden | The code diff adds no `difflib`, `get_close_matches`, `SequenceMatcher` or Levenshtein matching |
