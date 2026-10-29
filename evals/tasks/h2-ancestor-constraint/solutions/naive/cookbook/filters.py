"""Search filters. Each takes a list of recipes and returns the ones to keep."""

from typing import Dict, List

from .recipes import Recipe

NOT_VEGETARIAN = {"meat", "fish"}


def by_cuisine(recipes: List[Recipe], cuisine: str) -> List[Recipe]:
    wanted = cuisine.lower()
    return [r for r in recipes if r.cuisine == wanted]


def _is_vegetarian(recipe: Recipe, ingredients: Dict[str, dict]) -> bool:
    for ingredient in recipe.ingredients:
        tags = set(ingredients.get(ingredient.name, {}).get("tags", []))
        if tags & NOT_VEGETARIAN:
            return False
    return True


def vegetarian_only(recipes: List[Recipe], ingredients: Dict[str, dict]) -> List[Recipe]:
    return [r for r in recipes if _is_vegetarian(r, ingredients)]


def within_minutes(recipes: List[Recipe], limit: int) -> List[Recipe]:
    return [r for r in recipes if r.minutes is not None and r.minutes <= limit]
