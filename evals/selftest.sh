#!/usr/bin/env bash
# Checks the graders without an agent. For each task it grades three workdirs:
#
#   untouched  the fixture as the agent gets it: base items for new behaviour fail
#   correct    tasks/<task>/solutions/correct copied over the fixture: every item passes
#   naive      tasks/<task>/solutions/naive: every base item passes, a hidden item fails
#
# Usage: selftest.sh [task...]. Exits 1 if any expectation fails.
set -u
EVALS=$(cd "$(dirname "$0")" && pwd)
. "$EVALS/lib/workdir.sh"

tasks=("$@")
[ ${#tasks[@]} -eq 0 ] && tasks=($(cd "$EVALS/tasks" && ls))
scratch=$(mktemp -d "${TMPDIR:-/tmp}/mfd-eval-selftest.XXXXXX")
failures=0

expect() {
  local kind=$1 json=$2
  python3 - "$kind" "$json" <<'PY'
import json, sys
kind, r = sys.argv[1], json.loads(sys.argv[2])
hidden = set(r["hidden"])
failed = set(r["failed"])
if kind == "correct":
    ok = not failed
elif kind == "naive":
    ok = bool(failed) and failed <= hidden
else:
    ok = bool(failed)
sys.exit(0 if ok else 1)
PY
}

for task in "${tasks[@]}"; do
  for kind in untouched correct naive; do
    work="$scratch/$task-$kind"
    make_workdir A "$work" || exit 2
    [ "$kind" != untouched ] && rsync -a "$EVALS/tasks/$task/solutions/$kind/" "$work/"
    json=$("$EVALS/tasks/$task/check.sh" "$work" "selftest-$kind" 0)
    if expect "$kind" "$json"; then verdict=ok; else verdict=UNEXPECTED; failures=$((failures + 1)); fi
    printf '%-10s %-24s %-9s %s\n' "$verdict" "$task" "$kind" "$json"
  done
done

rm -rf "$scratch"
[ "$failures" -eq 0 ] || { echo "$failures unexpected results" >&2; exit 1; }
echo "all graders behave as expected"
