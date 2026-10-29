"""The shopping list: every ingredient of the chosen recipes, combined and in aisle order."""

from typing import Dict, List, Tuple

from .recipes import Ingredient, Recipe

AISLES = ["produce", "bakery", "meat-fish", "dairy-eggs", "pantry"]


def combine(recipes: List[Recipe]) -> List[Ingredient]:
    totals: Dict[Tuple[str, str], float] = {}
    for recipe in recipes:
        for i in recipe.ingredients:
            key = (i.name, i.unit)
            totals[key] = totals.get(key, 0) + i.quantity
    return [Ingredient(name, quantity, unit) for (name, unit), quantity in totals.items()]


def in_aisle_order(items: List[Ingredient], ingredients: Dict[str, dict]) -> List[Ingredient]:
    def rank(item: Ingredient) -> int:
        aisle = ingredients.get(item.name, {}).get("aisle")
        return AISLES.index(aisle) if aisle in AISLES else len(AISLES)

    return sorted(items, key=rank)


def shopping_list(recipes: List[Recipe], ingredients: Dict[str, dict]) -> List[Ingredient]:
    return in_aisle_order(combine(recipes), ingredients)
