"""Average ratings from the website export in data/ratings.json."""

import json
from pathlib import Path
from typing import Dict, List

from .recipes import DATA

MIN_RATINGS = 3


def load_ratings(path: Path = DATA / "ratings.json") -> Dict[str, List[int]]:
    with open(path) as f:
        return json.load(f)


def format_rating(ratings: List[int]) -> str:
    if len(ratings) < MIN_RATINGS:
        return "(not enough ratings)"
    average = sum(ratings) / len(ratings)
    return f"★ {average:.1f} ({len(ratings)})"
