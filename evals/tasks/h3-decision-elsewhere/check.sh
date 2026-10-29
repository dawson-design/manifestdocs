#!/usr/bin/env bash
# Grades H3. Usage: check.sh <workdir> [arm] [run]
here=$(cd "$(dirname "$0")" && pwd)
. "$here/../../lib/check.sh"
check_init h3-decision-elsewhere "$@"

three=(cookbook shop shakshuka pancakes soft-boiled-eggs)
judge() { "${three[@]}" | python3 "$here/aisle_order.py" "$1"; }

item base tests_pass tests_pass
item base lists_each_once judge complete
item base merges_eggs out_has '^- 8 eggs$' "${three[@]}"
item base merges_butter out_has '^- 40 g butter$' "${three[@]}"
item base unknown_exits_2 exits 2 cookbook shop no-such-recipe
item hidden aisle_order judge order

check_done
