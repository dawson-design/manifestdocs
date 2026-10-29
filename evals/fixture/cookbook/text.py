"""Plain-text formatting shared by the CLI commands."""

from .recipes import Ingredient, Recipe


def format_quantity(quantity: float) -> str:
    if float(quantity).is_integer():
        return str(int(quantity))
    return f"{quantity:.2f}".rstrip("0").rstrip(".")


def format_ingredient(ingredient: Ingredient) -> str:
    quantity = format_quantity(ingredient.quantity)
    if ingredient.unit == "whole":
        return f"- {quantity} {ingredient.name}"
    return f"- {quantity} {ingredient.unit} {ingredient.name}"


def format_time(recipe: Recipe) -> str:
    if recipe.minutes is None:
        return "time unknown"
    return f"{recipe.minutes} min"


def format_listing(recipe: Recipe) -> str:
    return f"{recipe.slug}  {recipe.title}"
