#!/usr/bin/env bash
# Grades H5. Usage: check.sh <workdir> [arm] [run]
. "$(cd "$(dirname "$0")/../.." && pwd)/lib/check.sh"
check_init h5-control "$@"

item base tests_pass tests_pass
item base shows_average out_has '^shakshuka  Shakshuka  ★ 4\.3 \(3\)$' cookbook list
item base counts_ratings out_has '^pasta-pomodoro  Pasta al pomodoro  ★ 4\.3 \(12\)$' cookbook list
item base rounds_to_one_decimal out_has '^chicken-tikka-masala  Chicken tikka masala  ★ 4\.7 \(7\)$' cookbook list
item hidden too_few_ratings out_has '^pancakes  Pancakes  \(not enough ratings\)$' cookbook list
item hidden no_ratings out_has '^baba-ganoush  Baba ganoush  \(not enough ratings\)$' cookbook list

check_done
