#!/usr/bin/env bash
# Grades H2. Usage: check.sh <workdir> [arm] [run]
. "$(cd "$(dirname "$0")/../.." && pwd)/lib/check.sh"
check_init h2-ancestor-constraint "$@"

item base tests_pass tests_pass
item base keeps_pancakes out_has '^pancakes ' cookbook search --max-minutes 30
item base keeps_tacos_at_limit out_has '^fish-tacos ' cookbook search --max-minutes 30
item base drops_shakshuka out_lacks '^shakshuka ' cookbook search --max-minutes 30
item base works_with_query out_has '^soft-boiled-eggs ' cookbook search eggs --max-minutes 15
item base zero_exits_2 exits 2 cookbook search --max-minutes 0
item hidden keeps_unknown_stew out_has '^grandmas-beef-stew ' cookbook search --max-minutes 30
item hidden keeps_unknown_bread out_has '^banana-bread ' cookbook search --max-minutes 30

check_done
