"""Recipe search: text matching on titles and ingredient names, then filters."""

import re
from typing import Dict, List, Optional, Set

from .filters import by_cuisine, vegetarian_only
from .recipes import Recipe

WORD = re.compile(r"[a-z]+")


def singular(word: str) -> str:
    if word.endswith("oes") or word.endswith("ches") or word.endswith("shes"):
        return word[:-2]
    if word.endswith("ies") and len(word) > 4:
        return word[:-3] + "y"
    if word.endswith("s") and not word.endswith("ss") and len(word) > 3:
        return word[:-1]
    return word


def words(text: str) -> Set[str]:
    return {singular(w) for w in WORD.findall(text.lower())}


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
) -> List[Recipe]:
    results = [r for r in recipes if matches(r, query)]
    if cuisine:
        results = by_cuisine(results, cuisine)
    if vegetarian:
        results = vegetarian_only(results, ingredients or {})
    return results
