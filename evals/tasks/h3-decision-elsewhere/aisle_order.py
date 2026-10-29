"""Judges the output of `cookbook shop shakshuka pancakes soft-boiled-eggs` on stdin.

`order` (the default) exits 0 when the ingredient lines follow the store's aisle order.
`complete` exits 0 when every ingredient appears on exactly one line. Lines that don't
start with "- ", such as aisle headings, are ignored. The aisles are the grader's own
copy, so an agent editing data/ingredients.json can't change the answer.
"""

import sys

ORDER = ["produce", "bakery", "meat-fish", "dairy-eggs", "pantry"]
AISLE = {
    "tomatoes": "produce",
    "onion": "produce",
    "bread slices": "bakery",
    "eggs": "dairy-eggs",
    "feta": "dairy-eggs",
    "milk": "dairy-eggs",
    "butter": "dairy-eggs",
    "olive oil": "pantry",
    "cumin": "pantry",
    "flour": "pantry",
    "sugar": "pantry",
}


def ingredient(line: str) -> str:
    matches = [name for name in AISLE if line.rstrip().endswith(name)]
    return max(matches, key=len) if matches else ""


def main(mode: str) -> int:
    lines = [l for l in sys.stdin.read().splitlines() if l.startswith("- ")]
    names = [ingredient(l) for l in lines]
    if mode == "complete":
        # Every ingredient, each on one line, and nothing else.
        return 0 if sorted(names) == sorted(AISLE) else 1
    names = [n for n in names if n]
    if len(names) < 5:
        print(f"too few ingredient lines to judge: {lines}", file=sys.stderr)
        return 1
    ranks = [ORDER.index(AISLE[n]) for n in names]
    if ranks != sorted(ranks):
        print(f"out of aisle order: {names}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "order"))
