import unittest

from cookbook.recipes import load_ingredients, load_recipes
from cookbook.search import search


def slugs(results):
    return {r.slug for r in results}


class SearchTest(unittest.TestCase):
    def setUp(self):
        self.recipes = load_recipes()
        self.ingredients = load_ingredients()

    def test_empty_query_returns_everything(self):
        self.assertEqual(len(search(self.recipes)), len(self.recipes))

    def test_matches_title(self):
        self.assertEqual(slugs(search(self.recipes, "shakshuka")), {"shakshuka"})

    def test_matches_ingredient(self):
        self.assertIn("red-lentil-dal", slugs(search(self.recipes, "turmeric")))

    def test_every_word_must_match(self):
        found = slugs(search(self.recipes, "olive oil feta"))
        self.assertEqual(found, {"shakshuka", "greek-salad"})

    def test_is_case_insensitive(self):
        self.assertEqual(slugs(search(self.recipes, "SHAKSHUKA")), {"shakshuka"})

    def test_filters_by_cuisine(self):
        found = slugs(search(self.recipes, cuisine="indian"))
        self.assertEqual(found, {"chicken-tikka-masala", "red-lentil-dal"})

    def test_vegetarian_leaves_out_meat_and_fish(self):
        found = slugs(search(self.recipes, vegetarian=True, ingredients=self.ingredients))
        self.assertNotIn("chicken-tikka-masala", found)
        self.assertNotIn("fish-tacos", found)
        self.assertNotIn("grandmas-beef-stew", found)
        self.assertIn("shakshuka", found)


if __name__ == "__main__":
    unittest.main()
