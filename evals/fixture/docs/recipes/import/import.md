---
id: fd-230023
title: "Import from recipe cards"
status: draft
summary: "Turns typed-up family recipe cards into catalogue entries, keeping gaps as unknown rather than guessing."
---

# Import from recipe cards

Readers send us photos of family recipe cards, and an editor types them up. Cards often leave out the cooking time, and sometimes the cuisine.

## Requirements

### Requirement: Keep gaps as unknown

An imported recipe SHALL record a missing total time as `null`, never an estimate.

#### Scenario: Card without a time

- WHEN an editor imports Grandma's beef stew, whose card gives no time
- THEN its `minutes` is `null`, and its recipe page reads `time unknown`

## Open questions

- Should imported recipes carry the name of the reader who sent the card?
- Do we need a review step before an imported recipe appears in `cookbook list`?
