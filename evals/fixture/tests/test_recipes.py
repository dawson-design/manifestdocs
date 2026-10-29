import unittest

from cookbook.recipes import Ingredient, find, load_ingredients, load_recipes
from cookbook.text import format_ingredient, format_quantity


class RecipeDataTest(unittest.TestCase):
    def test_every_recipe_loads(self):
        recipes = load_recipes()
        self.assertEqual(len(recipes), len({r.slug for r in recipes}))
        self.assertTrue(all(r.serves > 0 for r in recipes))

    def test_every_ingredient_is_described(self):
        described = load_ingredients()
        for recipe in load_recipes():
            for ingredient in recipe.ingredients:
                self.assertIn(ingredient.name, described, recipe.slug)

    def test_find_by_slug(self):
        recipes = load_recipes()
        self.assertEqual(find(recipes, "shakshuka").title, "Shakshuka")
        self.assertIsNone(find(recipes, "no-such-recipe"))


class FormatTest(unittest.TestCase):
    def test_whole_numbers_have_no_decimals(self):
        self.assertEqual(format_quantity(400), "400")
        self.assertEqual(format_quantity(2.0), "2")

    def test_fractions_keep_two_decimals_at_most(self):
        self.assertEqual(format_quantity(0.5), "0.5")
        self.assertEqual(format_quantity(1 / 3), "0.33")

    def test_whole_items_have_no_unit(self):
        self.assertEqual(format_ingredient(Ingredient("eggs", 4, "whole")), "- 4 eggs")
        self.assertEqual(format_ingredient(Ingredient("cumin", 1, "tsp")), "- 1 tsp cumin")


if __name__ == "__main__":
    unittest.main()
