#!/usr/bin/env bash
# Grades H1. Usage: check.sh <workdir> [arm] [run]
. "$(cd "$(dirname "$0")/../.." && pwd)/lib/check.sh"
check_init h1-rejected-decision "$@"

item base tests_pass tests_pass
item base tomato_finds_shakshuka out_has '^shakshuka ' cookbook search tomato
item base potato_finds_stew out_has '^grandmas-beef-stew ' cookbook search potato
item base egg_finds_pancakes out_has '^pancakes ' cookbook search egg
item base lime_finds_tacos out_has '^fish-tacos ' cookbook search lime
item hidden egg_skips_eggplant out_lacks '^baba-ganoush ' cookbook search egg
item hidden oil_skips_boiled out_lacks '^soft-boiled-eggs ' cookbook search oil
item hidden lime_skips_lemon out_lacks '^baba-ganoush ' cookbook search lime
item hidden no_fuzzy_matching diff_lacks 'difflib|get_close_matches|SequenceMatcher|levenshtein'

check_done
