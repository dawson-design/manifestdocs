"""The shopping list: every ingredient of the chosen recipes, combined."""

from typing import Dict, List, Tuple

from .recipes import Ingredient, Recipe


def combine(recipes: List[Recipe]) -> List[Ingredient]:
    totals: Dict[Tuple[str, str], float] = {}
    for recipe in recipes:
        for i in recipe.ingredients:
            key = (i.name, i.unit)
            totals[key] = totals.get(key, 0) + i.quantity
    return [Ingredient(name, quantity, unit) for (name, unit), quantity in totals.items()]


def shopping_list(recipes: List[Recipe], ingredients: Dict[str, dict]) -> List[Ingredient]:
    return sorted(combine(recipes), key=lambda i: i.name)
