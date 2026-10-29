# Ranking rules (draft)

These are the rules we're testing for ranking. None of them ships until the ranking node is approved.

1. A match in the title outranks a match only in the ingredients.
2. Among title matches, a shorter title ranks higher, so "Shakshuka" beats "Green shakshuka with feta".
3. Among ingredient matches, a recipe where the query is a main ingredient, by weight, ranks higher.
4. Ties keep catalogue order.

## Open questions

- Should a reader's own ratings, once we have them, lift a recipe? The search team rejected sorting by the website's ratings, so this needs a new reason.
- How do we weigh whole items, such as eggs, against grams for rule 3?
