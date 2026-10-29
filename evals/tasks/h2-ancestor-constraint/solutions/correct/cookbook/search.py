"""Recipe search: text matching on titles and ingredient names, then filters."""

import re
from typing import Dict, List, Optional, Set

from .filters import by_cuisine, vegetarian_only, within_minutes
from .recipes import Recipe

WORD = re.compile(r"[a-z]+")


def words(text: str) -> Set[str]:
    return set(WORD.findall(text.lower()))


def recipe_words(recipe: Recipe) -> Set[str]:
    found = words(recipe.title)
    for ingredient in recipe.ingredients:
        found |= words(ingredient.name)
    return found


def matches(recipe: Recipe, query: str) -> bool:
    return words(query) <= recipe_words(recipe)


def search(
    recipes: List[Recipe],
    query: str = "",
    cuisine: Optional[str] = None,
    vegetarian: bool = False,
    ingredients: Optional[Dict[str, dict]] = None,
    max_minutes: Optional[int] = None,
) -> List[Recipe]:
    results = [r for r in recipes if matches(r, query)]
    if cuisine:
        results = by_cuisine(results, cuisine)
    if vegetarian:
        results = vegetarian_only(results, ingredients or {})
    if max_minutes is not None:
        results = within_minutes(results, max_minutes)
    return results
