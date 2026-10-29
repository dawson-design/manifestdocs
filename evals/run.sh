#!/usr/bin/env bash
# Runs the pilot: for each task, arm and run, copies the fixture to a fresh git repo, runs the
# agent headless in a throwaway container that mounts only that repo, then grades the repo on
# the host and records retrieval metrics.
#
# Usage: run.sh [--dry-run] [--no-build] [--tasks t1,t2] [--arms A,B,C] [--runs N] [--agent NAME] [--out DIR]
#
#   --dry-run   build the images, set up every workdir and grade the untouched fixture, but
#               print the agent command instead of running it. Calls no model.
#   --no-build  use the mfd-eval:a and mfd-eval:b images as they are
#   --tasks     task folders under tasks/ (default: the first paid run, H2, H4 and H5;
#               H1 and H3 wait for decisions in `context`)
#   --arms      A (agent CLI only), B (plus manifestdocs and the plugin), C (B with context
#               injected) (default: A,B)
#   --runs      sessions per task and arm (default: 3)
#   --agent     an adapter in agents/ (default: claude)
#   --out       results folder (default: results/<timestamp>)
#
# Environment: ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN (passed into the container),
# EVAL_MODEL (default claude-opus-5-5), EVAL_MAX_USD (per session, default 3), EVAL_TIMEOUT
# (seconds per session, default 1800), EVAL_BUDGET (context budget for arm C, default 8000),
# EVAL_BUILD_ARGS (extra `docker build` arguments, such as a --build-arg for a base image).
set -u
EVALS=$(cd "$(dirname "$0")" && pwd)
. "$EVALS/lib/workdir.sh"

dry=0 build=1 runs=3 agent=claude arms=A,B out=""
tasks=h2-ancestor-constraint,h4-linked-doc,h5-control
while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) dry=1 ;;
    --no-build) build=0 ;;
    --tasks) tasks=$2; shift ;;
    --arms) arms=$2; shift ;;
    --runs) runs=$2; shift ;;
    --agent) agent=$2; shift ;;
    --out) out=$2; shift ;;
    *) sed -n '2,24p' "$0"; exit 2 ;;
  esac
  shift
done
[ -x "$EVALS/agents/$agent.sh" ] || { echo "no adapter: agents/$agent.sh" >&2; exit 2; }
out=${out:-$EVALS/results/$( [ $dry = 1 ] && echo dry- )$(date +%Y%m%d-%H%M%S)}
budget=${EVAL_BUDGET:-8000}
export EVAL_MODEL=${EVAL_MODEL:-claude-opus-5-5}

image_for() { [ "$1" = A ] && echo mfd-eval:a || echo mfd-eval:b; }
json_field() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))[sys.argv[2]])' "$1" "$2"; }
now() { python3 -c 'import time; print(time.time())'; }

require_docker() {
  docker info >/dev/null 2>&1 && return 0
  echo "Docker isn't installed or isn't running. Start Docker and rerun; run.sh doesn't install it." >&2
  exit 2
}

require_credentials() {
  [ -n "${ANTHROPIC_API_KEY:-}" ] || [ -n "${CLAUDE_CODE_OAUTH_TOKEN:-}" ] && return 0
  echo "Set ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN; the container gets credentials only from them." >&2
  exit 2
}

build_images() {
  local target
  for target in a b; do
    echo "building mfd-eval:$target"
    # EVAL_BUILD_ARGS can override the pinned base images, as in --build-arg NODE_IMAGE=...
    docker build -q -f "$EVALS/Dockerfile" --target "arm-$target" ${EVAL_BUILD_ARGS:-} \
      -t "mfd-eval:$target" "$REPO_ROOT" >/dev/null || exit 2
  done
  docker image ls --format '  {{.Repository}}:{{.Tag}}  {{.Size}}' mfd-eval
}

# Shows what each image gives the agent, without calling a model.
describe_images() {
  local image
  for image in mfd-eval:a mfd-eval:b; do
    echo "$image:"
    docker image inspect "$image" >/dev/null 2>&1 || { echo "  not built"; continue; }
    docker run --rm --network none --entrypoint sh "$image" -c \
      'echo "  manifestdocs: $(command -v manifestdocs || echo none)"; echo "  plugins:"; claude plugin list 2>&1 | sed "s/^/    /"'
  done
}

# Appends `manifestdocs context` for the task's node to the system prompt (arm C). It runs in
# the arm B image, with no network and the workdir mounted read-only.
inject_context() {
  local work=$1 system=$2 task=$3 node
  node=$(json_field "$EVALS/tasks/$task/task.json" node)
  printf '\n## Context for this task\n\nThe output of `manifestdocs context %s --format md --budget %s`, run when the session started:\n\n' "$node" "$budget" >>"$system"
  docker run --rm --network none --mount "type=bind,src=$work,dst=/work,readonly" --workdir /work \
    --entrypoint manifestdocs mfd-eval:b context "$node" --format md --budget "$budget" >>"$system"
}

session() {
  local task=$1 arm=$2 run=$3
  local id=$task.$arm.$run dir=$scratch/$task.$arm.$run
  local work=$dir/repo transcript=$out/$task.$arm.$run.jsonl start wall
  make_workdir "$arm" "$work" || exit 2
  cp "$work/AGENTS.md" "$dir/system.md"
  [ "$arm" = C ] && inject_context "$work" "$dir/system.md" "$task"

  echo "== $id"
  start=$(now)
  EVAL_IMAGE=$(image_for "$arm") EVAL_NAME=mfd-eval-$id EVAL_WORKDIR=$work \
    EVAL_PROMPT_FILE=$EVALS/tasks/$task/prompt.md EVAL_SYSTEM_FILE=$dir/system.md \
    EVAL_TRANSCRIPT=$transcript EVAL_DRY_RUN=$dry EVAL_MAX_USD=${EVAL_MAX_USD:-3} \
    EVAL_TIMEOUT=${EVAL_TIMEOUT:-1800} "$EVALS/agents/$agent.sh"
  wall=$(python3 -c "print(round($(now) - $start, 1))")

  record "$task" "$arm" "$run" "$dir" "$work" "$transcript" "$wall"
}

# Grades the workdir on the host, measures the transcript and writes $out/<id>.json and <id>.diff.
record() {
  local task=$1 arm=$2 run=$3 dir=$4 work=$5 transcript=$6 wall=$7 id=$1.$2.$3
  [ -s "$transcript" ] || transcript=""
  "$EVALS/tasks/$task/check.sh" "$work" "$arm" "$run" >"$dir/check.json"
  (cd "$work" && git add -A -N && git diff eval-base) >"$out/$id.diff"
  python3 "$EVALS/lib/metrics.py" --task-json "$EVALS/tasks/$task/task.json" --arm "$arm" \
    --wall "$wall" ${transcript:+--transcript "$transcript"} \
    $( [ "$arm" = C ] && echo --injected "$dir/system.md" ) >"$dir/metrics.json"
  python3 - "$dir/check.json" "$dir/metrics.json" "$agent" "$out/$id.json" <<'PY'
import json, sys
check, metrics, agent, dest = sys.argv[1:]
result = json.load(open(check))
result["agent"] = agent
result["metrics"] = json.load(open(metrics))
json.dump(result, open(dest, "w"), indent=2)
print("  graded:", json.dumps({k: result[k] for k in ("passed", "failed")}))
PY
}

require_docker
[ $dry = 1 ] || require_credentials
[ $build = 1 ] && build_images
[ $dry = 1 ] && describe_images
mkdir -p "$out"
scratch=$(mktemp -d "${TMPDIR:-/tmp}/mfd-eval.XXXXXX")
echo "model: $EVAL_MODEL"
echo "results: $out"
echo "workdirs: $scratch"

IFS=, read -r -a task_list <<<"$tasks"
IFS=, read -r -a arm_list <<<"$arms"
for task in "${task_list[@]}"; do
  [ -f "$EVALS/tasks/$task/check.sh" ] || { echo "no task: $task" >&2; exit 2; }
  for arm in "${arm_list[@]}"; do
    for run in $(seq 1 "$runs"); do session "$task" "$arm" "$run"; done
  done
done
