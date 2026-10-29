---
id: fd-0da3f4
title: "Ratings"
status: approved
summary: "Shows each recipe's average rating from the website in `cookbook list`."
---

# Ratings

Readers rate recipes on the website, from 1 to 5 stars. The website exports the ratings to `data/ratings.json`, which maps each recipe slug to its list of ratings. A recipe with no ratings may be missing from the file.

## Requirements

### Requirement: Average rating in the list

`cookbook list` SHALL show each recipe's average rating after its title: two spaces, a star, the average to one decimal place, and the number of ratings in brackets.

#### Scenario: Three ratings

- WHEN Shakshuka has ratings of 5, 4 and 4
- THEN its line reads `shakshuka  Shakshuka  ★ 4.3 (3)`

### Requirement: Too few ratings

A recipe with fewer than 3 ratings SHALL show `(not enough ratings)` after its title instead of an average, because one or two ratings mislead readers.

#### Scenario: Two ratings

- WHEN Pancakes has two ratings of 5
- THEN its line reads `pancakes  Pancakes  (not enough ratings)`

#### Scenario: No ratings

- WHEN Baba ganoush has no ratings
- THEN its line reads `baba-ganoush  Baba ganoush  (not enough ratings)`
