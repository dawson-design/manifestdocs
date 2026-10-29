# Shared helpers for each task's check.sh. Source it, then:
#
#   check_init <task> <workdir> [arm] [run]
#   item <base|hidden> <id> <command...>   # passes when the command exits 0
#   check_done                             # prints the JSON result
#
# `hidden` marks the items that hinge on the task's hidden fact; `base` items come from
# the prompt or the target node's own scenarios.

export PYTHONDONTWRITEBYTECODE=1

check_init() {
  TASK=$1
  WORK=$2
  ARM=${3:-none}
  RUN=${4:-0}
  PASSED=()
  FAILED=()
  HIDDEN=()
  cd "$WORK" || { echo "no workdir: $WORK" >&2; exit 2; }
  DIFF=$(mktemp)
  # Intent-to-add makes new files show in the diff without staging their content.
  git add -A -N >/dev/null 2>&1
  git diff eval-base -- . ':(exclude)docs' >"$DIFF" 2>/dev/null
}

item() {
  local kind=$1 id=$2
  shift 2
  [ "$kind" = hidden ] && HIDDEN+=("$id")
  if "$@" >/dev/null 2>&1; then PASSED+=("$id"); else FAILED+=("$id"); fi
}

check_done() {
  rm -f "$DIFF"
  python3 - "$TASK" "$ARM" "$RUN" "${#PASSED[@]}" "${PASSED[@]}" "${#FAILED[@]}" "${FAILED[@]}" "${HIDDEN[@]}" <<'PY'
import json, sys
a = sys.argv[1:]
task, arm, run = a[0], a[1], int(a[2])
n = int(a[3]); passed = a[4:4 + n]; a = a[4 + n:]
m = int(a[0]); failed = a[1:1 + m]; hidden = a[1 + m:]
print(json.dumps({"task": task, "arm": arm, "run": run, "passed": passed, "failed": failed, "hidden": hidden}))
PY
}

# Runs a command with a time limit, so a hung solution can't stall the pilot.
with_timeout() {
  local secs=$1
  shift
  perl -e 'alarm shift; exec @ARGV' "$secs" "$@"
}

cookbook() { with_timeout 30 python3 -m cookbook "$@"; }

tests_pass() { with_timeout 180 python3 -m unittest -q; }

# out_has REGEX cmd...: the command exits 0 and a line of its stdout matches REGEX.
out_has() {
  local re=$1 out
  shift
  out=$("$@") || return 1
  printf '%s\n' "$out" | grep -Eq -- "$re"
}

# out_lacks REGEX cmd...: the command exits 0 and no line of its stdout matches REGEX.
out_lacks() {
  local re=$1 out
  shift
  out=$("$@") || return 1
  ! printf '%s\n' "$out" | grep -Eq -- "$re"
}

# exits CODE cmd...: the command exits with CODE.
exits() {
  local want=$1
  shift
  "$@"
  [ $? -eq "$want" ]
}

# The diff of the code, against the fixture as committed before the agent ran.
diff_has() { grep -E '^\+' "$DIFF" | grep -Eq -- "$1"; }
diff_lacks() { ! diff_has "$1"; }
