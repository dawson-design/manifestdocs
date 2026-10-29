import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def cookbook(*args):
    return subprocess.run(
        [sys.executable, "-m", "cookbook", *args],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=20,
    )


class CliTest(unittest.TestCase):
    def test_list_prints_slug_and_title(self):
        out = cookbook("list")
        self.assertEqual(out.returncode, 0)
        self.assertIn("shakshuka  Shakshuka  ★ 4.3 (3)", out.stdout.splitlines())

    def test_show_prints_header_and_ingredients(self):
        out = cookbook("show", "shakshuka")
        self.assertEqual(out.returncode, 0)
        lines = out.stdout.splitlines()
        self.assertEqual(lines[0], "Shakshuka")
        self.assertEqual(lines[1], "Serves 2 · 35 min")
        self.assertIn("- 4 eggs", lines)
        self.assertIn("- 400 g tomatoes", lines)

    def test_show_unknown_time(self):
        out = cookbook("show", "banana-bread")
        self.assertIn("Serves 8 · time unknown", out.stdout)

    def test_show_unknown_recipe_exits_2(self):
        out = cookbook("show", "no-such-recipe")
        self.assertEqual(out.returncode, 2)
        self.assertIn("no recipe called", out.stderr)

    def test_search_prints_matches(self):
        out = cookbook("search", "lentils")
        self.assertEqual(out.returncode, 0)
        self.assertEqual(out.stdout.splitlines(), ["red-lentil-dal  Red lentil dal"])

    def test_search_with_no_matches(self):
        out = cookbook("search", "durian")
        self.assertEqual(out.stdout.strip(), "No recipes found.")


if __name__ == "__main__":
    unittest.main()
