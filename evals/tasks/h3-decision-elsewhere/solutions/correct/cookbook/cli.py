"""The `cookbook` command line: list, show and search."""

import argparse
import sys
from typing import List, Optional

from .recipes import find, load_ingredients, load_recipes
from .search import search
from .shopping import shopping_list
from .text import format_ingredient, format_listing, format_time


def cmd_list(args: argparse.Namespace) -> int:
    for recipe in load_recipes():
        print(format_listing(recipe))
    return 0


def cmd_show(args: argparse.Namespace) -> int:
    recipe = find(load_recipes(), args.slug)
    if recipe is None:
        print(f"error: no recipe called {args.slug!r}", file=sys.stderr)
        return 2
    print(recipe.title)
    print(f"Serves {recipe.serves} · {format_time(recipe)}")
    print()
    for ingredient in recipe.ingredients:
        print(format_ingredient(ingredient))
    return 0


def cmd_search(args: argparse.Namespace) -> int:
    results = search(
        load_recipes(),
        query=args.query,
        cuisine=args.cuisine,
        vegetarian=args.vegetarian,
        ingredients=load_ingredients(),
    )
    if not results:
        print("No recipes found.")
    for recipe in results:
        print(format_listing(recipe))
    return 0


def cmd_shop(args: argparse.Namespace) -> int:
    recipes = load_recipes()
    chosen = []
    for slug in args.slugs:
        recipe = find(recipes, slug)
        if recipe is None:
            print(f"error: no recipe called {slug!r}", file=sys.stderr)
            return 2
        chosen.append(recipe)
    for item in shopping_list(chosen, load_ingredients()):
        print(format_ingredient(item))
    return 0


def parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="cookbook")
    sub = p.add_subparsers(dest="command", required=True)

    sub.add_parser("list", help="list every recipe").set_defaults(run=cmd_list)

    show = sub.add_parser("show", help="show one recipe")
    show.add_argument("slug")
    show.set_defaults(run=cmd_show)

    find_ = sub.add_parser("search", help="search recipes by title and ingredient")
    find_.add_argument("query", nargs="?", default="")
    find_.add_argument("--cuisine")
    find_.add_argument("--vegetarian", action="store_true")
    find_.set_defaults(run=cmd_search)

    shop = sub.add_parser("shop", help="one shopping list for several recipes")
    shop.add_argument("slugs", nargs="+")
    shop.set_defaults(run=cmd_shop)
    return p


def main(argv: Optional[List[str]] = None) -> int:
    args = parser().parse_args(argv)
    return args.run(args)
