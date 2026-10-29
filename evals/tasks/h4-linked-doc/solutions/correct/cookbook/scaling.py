"""Scales a recipe to a number of servings, rounding to quantities a cook can measure."""

import math
from dataclasses import replace

from .recipes import Ingredient, Recipe

MAX_SERVINGS = 24


def measurable(quantity: float, unit: str) -> float:
    if unit == "whole":
        return float(math.ceil(quantity - 1e-9))
    if unit in ("tsp", "tbsp"):
        return round(quantity * 4) / 4
    return float(round(quantity))


def scale(recipe: Recipe, servings: int) -> Recipe:
    factor = servings / recipe.serves
    ingredients = tuple(
        Ingredient(i.name, measurable(i.quantity * factor, i.unit), i.unit)
        for i in recipe.ingredients
    )
    return replace(recipe, serves=servings, ingredients=ingredients)
