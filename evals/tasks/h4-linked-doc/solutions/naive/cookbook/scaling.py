"""Scales a recipe to a number of servings."""

from dataclasses import replace

from .recipes import Ingredient, Recipe

MAX_SERVINGS = 24


def scale(recipe: Recipe, servings: int) -> Recipe:
    factor = servings / recipe.serves
    ingredients = tuple(
        Ingredient(i.name, i.quantity * factor, i.unit) for i in recipe.ingredients
    )
    return replace(recipe, serves=servings, ingredients=ingredients)
