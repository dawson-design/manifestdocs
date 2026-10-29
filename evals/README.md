# Retrieval eval (pilot)

Does Manifest Docs help a coding agent find the context a change depends on, compared with plain bash? Each task is a realistic issue against a small cookbook project whose correct answer hinges on one fact in the docs tree that the issue doesn't mention. The pilot measures whether the agent found that fact, whether its change respects it, and what the search cost.

## Hypotheses

| Task | Hypothesis | Where the hidden fact lives | In `mfd context` today? |
|---|---|---|---|
| `h1-rejected-decision` | A decision record that rejected an approach stops the agent reintroducing it | The target node's `decisions/` | No, until A3 |
| `h2-ancestor-constraint` | A rule on a parent node reaches work on a child that doesn't repeat it | The parent node's body | Yes |
| `h3-decision-elsewhere` | A decision placed on the root by the nearest-common-ancestor rule reaches the nodes it governs | The root's `decisions/` | No, until A3 |
| `h4-linked-doc` | A detail in a supporting doc linked from the node reaches the agent | A linked doc, mid-page | Yes |
| `h5-control` | Nothing to retrieve beyond the node the issue names, so the arms should tie | The target node | Yes |

Each task's `checklist.md` states the fact, the obvious wrong implementation and every graded item. `run.sh` runs H2, H4 and H5 by default. H1 and H3 wait until `context` includes decisions (A3); run them with `--tasks`.

## Arms

Every arm gets the same `AGENTS.md`, passed with `--append-system-prompt`, which says the requirements and decisions are in `docs/`.

- **A, bash.** Image `mfd-eval:a`: Claude Code, Python, git. No `manifestdocs`, no plugin.
- **B, mfd.** Image `mfd-eval:b`: A, plus `manifestdocs` and `mfd` built for Linux, and the plugin from `plugins/manifestdocs` installed from the repo's local marketplace, as a user installs it. The workdir also gets a `.manifestdocs/config.yaml` pointer to `docs/`.
- **C, context at start.** B, plus `manifestdocs context <node> --format md --budget 8000` appended to the system prompt. Opt in with `--arms A,B,C`.

`metrics.py` confirms from each transcript that arm B loaded the plugin and got its session-start outline, and that arm A did neither; the report warns otherwise.

## Isolation

Each session runs in a throwaway container (`docker run --rm`) whose only mount is that session's fresh repo, at `/work`. The image holds the agent CLI and its runtime, and for B the binaries and plugin; `evals/Dockerfile.dockerignore` keeps `evals/`, with its solutions, checklists and graders, out of every image. `bypassPermissions` applies inside the container only. Credentials come from `ANTHROPIC_API_KEY` or `CLAUDE_CODE_OAUTH_TOKEN` in your environment, passed with `-e`; no credential file is mounted. Grading runs on the host after the container exits. The container keeps outbound network access, which the agent needs for the API.

## Layout

```
Dockerfile          images mfd-eval:a and mfd-eval:b; build context is the repo root
fixture/            the cookbook: Python 3.9 stdlib code, tests, data/ and a 17-node tree in docs/
tasks/<task>/       prompt.md, checklist.md, check.sh, task.json (node, needed files, fact sentinels),
                    solutions/{correct,naive}/ (hand-made overlays for selftest.sh)
agents/claude.sh    the agent adapter
lib/                workdir setup, grading helpers, transcript metrics
run.sh              runs sessions and grades them
report.py           aggregates a results folder
selftest.sh         checks the graders without an agent
```

## Check the harness (no model calls)

```sh
evals/selftest.sh                                 # each check.sh: correct passes, naive fails a hidden item
evals/run.sh --dry-run --arms A,B,C --runs 1      # builds the images, prints each docker command, grades the untouched fixture
```

## Run the pilot

Docker must be running; `run.sh` stops if it isn't. Run one session per arm first, to confirm the plugin checks and what a session really costs:

```sh
export ANTHROPIC_API_KEY=...        # or CLAUDE_CODE_OAUTH_TOKEN from `claude setup-token`
evals/run.sh --tasks h2-ancestor-constraint --runs 1
python3 evals/report.py evals/results/<timestamp>
```

Then the first paid run, H2, H4 and H5 × arms A and B × 3 runs, 18 sessions:

```sh
evals/run.sh
```

`EVAL_MODEL` pins the model, `claude-opus-5-5` by default. `EVAL_MAX_USD` caps each session (default $3) and `EVAL_TIMEOUT` its wall time (default 1,800 s). Sessions run one at a time.

**Cost.** 18 sessions; a session on this fixture should take 15 to 40 tool calls and 2 to 6 minutes. Take the price from the smoke test's `cost_usd`. The per-session cap bounds the run at $54. On a subscription token, the sessions use plan quota instead.

## Read the results

`report.py` prints a table per task:

- **Solved:** runs that passed every checklist item.
- **Checklist:** the share of items passed, mean and min–max over runs.
- **Hidden:** the share of hidden-fact items passed, counted only in runs that passed every base item, because some hidden items also pass on the untouched fixture.
- **Facts seen:** runs where the fact's sentence reached the agent, in a tool result or the injected context. Low **Hidden** with high **Facts seen** means the agent found the fact and ignored it, which is a different problem from not finding it.
- **Calls, Tokens, Cost, Wall:** medians, the retrieval cost.

Rules for reading it:

- A difference no larger than the run-to-run spread is no difference. With 3 runs this is a coarse filter; the pilot's job is to show which hypotheses are worth more runs.
- H5 should tie. If it doesn't, something other than retrieval differs between the arms, and the other results can't be trusted.
- A task whose prompt hints at its answer proves nothing. Check any task where every arm scores 100% for a hint before reading it as a tie.

## Add another agent

Add the agent CLI to the Dockerfile, then write `agents/<name>.sh` that reads the `EVAL_*` variables listed in `agents/claude.sh`, runs the agent in a container on `EVAL_IMAGE` with `EVAL_WORKDIR` mounted at `/work`, and writes its transcript to `EVAL_TRANSCRIPT`. Pass `--agent <name>`. Grading works for any agent. `lib/metrics.py` reads only Claude Code's stream-json format, so another agent needs its own parser for the retrieval columns.
