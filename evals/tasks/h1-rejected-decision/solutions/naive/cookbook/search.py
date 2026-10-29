"""Recipe search: text matching on titles and ingredient names, then filters."""

from typing import Dict, List, Optional

from .filters import by_cuisine, vegetarian_only
from .recipes import Recipe


def recipe_text(recipe: Recipe) -> str:
    return " ".join([recipe.title] + [i.name for i in recipe.ingredients]).lower()


def matches(recipe: Recipe, query: str) -> bool:
    text = recipe_text(recipe)
    return all(word in text for word in query.lower().split())


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
