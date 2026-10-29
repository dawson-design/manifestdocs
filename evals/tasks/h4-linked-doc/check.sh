#!/usr/bin/env bash
# Grades H4. Usage: check.sh <workdir> [arm] [run]
. "$(cd "$(dirname "$0")/../.." && pwd)/lib/check.sh"
check_init h4-linked-doc "$@"

quarter='(0\.75|¾|3/4)'

item base tests_pass tests_pass
item base header_shows_servings out_has '^Serves 4 · 35 min$' cookbook show shakshuka --servings 4
item base doubles_grams out_has '^- 800 g tomatoes$' cookbook show shakshuka --servings 4
item base doubles_eggs out_has '^- 8 eggs$' cookbook show shakshuka --servings 4
item base over_24_exits_2 exits 2 cookbook show pancakes --servings 30
item base zero_exits_2 exits 2 cookbook show pancakes --servings 0
item hidden eggs_round_up out_has '^- 2 eggs$' cookbook show pancakes --servings 3
item hidden onion_rounds_up out_has '^- 2 onion$' cookbook show shakshuka --servings 3
item hidden grams_are_whole out_has '^- 2[23] g butter$' cookbook show pancakes --servings 3
item hidden spoons_to_quarter out_has "^- $quarter tsp mustard\$" cookbook show potato-salad --servings 4
item hidden spoons_to_quarter_above_one out_has '^- (2\.75|2 ¾|2 3/4) tbsp mayonnaise$' cookbook show potato-salad --servings 4

check_done
