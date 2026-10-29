"""Recipes and ingredients, loaded from the JSON files in data/."""

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Tuple

DATA = Path(__file__).resolve().parent.parent / "data"
UNITS = ("g", "ml", "tsp", "tbsp", "whole")


@dataclass(frozen=True)
class Ingredient:
    name: str
    quantity: float
    unit: str


@dataclass(frozen=True)
class Recipe:
    slug: str
    title: str
    cuisine: str
    serves: int
    minutes: Optional[int]
    ingredients: Tuple[Ingredient, ...]


def _ingredient(raw: dict) -> Ingredient:
    if raw["unit"] not in UNITS:
        raise ValueError(f"unknown unit {raw['unit']!r} for {raw['name']}")
    return Ingredient(raw["name"], raw["quantity"], raw["unit"])


def _recipe(raw: dict) -> Recipe:
    return Recipe(
        slug=raw["slug"],
        title=raw["title"],
        cuisine=raw["cuisine"],
        serves=raw["serves"],
        minutes=raw.get("minutes"),
        ingredients=tuple(_ingredient(i) for i in raw["ingredients"]),
    )


def load_recipes(path: Path = DATA / "recipes.json") -> List[Recipe]:
    with open(path) as f:
        return [_recipe(raw) for raw in json.load(f)]


def load_ingredients(path: Path = DATA / "ingredients.json") -> Dict[str, dict]:
    with open(path) as f:
        return json.load(f)


def find(recipes: List[Recipe], slug: str) -> Optional[Recipe]:
    return next((r for r in recipes if r.slug == slug), None)
