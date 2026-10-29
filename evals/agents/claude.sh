#!/usr/bin/env bash
# Runs Claude Code headless for one eval session, in a throwaway container. run.sh sets:
#
#   EVAL_IMAGE        mfd-eval:a for arm A; mfd-eval:b (manifestdocs and the plugin) otherwise
#   EVAL_NAME         the container's name
#   EVAL_WORKDIR      the fresh fixture repo, the container's only mount, at /work
#   EVAL_PROMPT_FILE  the task prompt
#   EVAL_SYSTEM_FILE  AGENTS.md, plus the injected context in arm C
#   EVAL_TRANSCRIPT   where to write the stream-json transcript, on the host
#   EVAL_DRY_RUN      1 to print the command instead of running it
#   EVAL_MODEL        model ID
#   EVAL_MAX_USD      spend cap for the session
#   EVAL_TIMEOUT      seconds before `timeout` stops the agent inside the container
#
# Credentials come from ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN in the host environment.
# `-e NAME` passes a variable's value without putting it on the command line, and passes
# nothing when it's unset. bypassPermissions applies inside the container only. The tool list
# is fixed and has no subagents, so every docs read shows up in this transcript; Skill is in
# it so arm B can use the plugin's skills.
set -u

docker_flags=(
  run --rm --init --name "$EVAL_NAME"
  --mount "type=bind,src=$EVAL_WORKDIR,dst=/work" --workdir /work
  -e ANTHROPIC_API_KEY -e CLAUDE_CODE_OAUTH_TOKEN
  --cap-drop ALL --security-opt no-new-privileges --pids-limit 512 --memory 4g
  --entrypoint timeout "$EVAL_IMAGE" "${EVAL_TIMEOUT:-1800}" claude
)
claude_flags=(
  --output-format stream-json --verbose --include-hook-events
  --strict-mcp-config --mcp-config '{"mcpServers":{}}'
  --tools "Bash,Read,Edit,Write,Glob,Grep,Skill"
  --permission-mode bypassPermissions
  --no-session-persistence
  --model "${EVAL_MODEL:-claude-opus-5-5}"
  --max-budget-usd "${EVAL_MAX_USD:-3}"
)

if [ "${EVAL_DRY_RUN:-0}" = 1 ]; then
  printf 'docker'
  printf ' %q' "${docker_flags[@]}"
  printf ' -p "$(cat %s)"' "$EVAL_PROMPT_FILE"
  printf ' %q' "${claude_flags[@]}"
  printf ' --append-system-prompt "$(cat %s)" </dev/null > %s\n' "$EVAL_SYSTEM_FILE" "$EVAL_TRANSCRIPT"
  exit 0
fi

docker "${docker_flags[@]}" -p "$(cat "$EVAL_PROMPT_FILE")" "${claude_flags[@]}" \
  --append-system-prompt "$(cat "$EVAL_SYSTEM_FILE")" \
  </dev/null >"$EVAL_TRANSCRIPT" 2>"$EVAL_TRANSCRIPT.stderr"
