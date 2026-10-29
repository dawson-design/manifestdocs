"""Retrieval metrics from one Claude Code `--output-format stream-json --verbose` transcript.

Usage: metrics.py --task-json T --arm A [--transcript F] [--injected F] [--wall SECONDS]

Prints one JSON object. With no transcript (a dry run), the transcript fields are null.

- tool_calls, tools: every tool_use block the agent made, and counts by tool name.
- mfd_calls: Bash commands that ran manifestdocs or mfd.
- docs_opened: docs/ Markdown paths named in Read, Grep or Glob inputs or in Bash commands.
- needed_opened: which of the task's needed_files are in docs_opened.
- facts_seen: for each hidden fact, whether its sentinel sentence reached the agent, in a tool
  result or in text injected into the system prompt (arm C). This is the main retrieval measure.
- facts_injected: for each hidden fact, whether arm C's injected context carried it. Measured
  without a transcript too, so a dry run shows what arm C starts with.
- plugin_loaded: the init event lists something named manifestdocs (a plugin, skill or slash
  command). Arm B must have it and arm A must not.
- outline_seen: the plugin's session-start hook put the tree outline in the session. It's
  looked for outside tool results, so the agent running `manifestdocs tree` doesn't count.
"""

import argparse
import json
import re
import sys
from collections import Counter
from typing import Dict, Iterator, List, Optional

DOCS_PATH = re.compile(r"(?:^|[\s/'\"=])(docs/[\w./-]+\.md)")
MFD_COMMAND = re.compile(r"(?:^|[\s;&|(])(?:\S*/)?(?:manifestdocs|mfd)\s")
# A line of `manifestdocs tree --depth 1` on the fixture: the root node with its ID.
OUTLINE = "cookbook (fd-3e9d0a"


def events(path: str) -> Iterator[dict]:
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line.startswith("{"):
                try:
                    yield json.loads(line)
                except json.JSONDecodeError:
                    continue


def normalise(text: str) -> str:
    return " ".join(text.split()).lower()


def result_text(content) -> str:
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "\n".join(c.get("text", "") for c in content if isinstance(c, dict))
    return ""


def blocks(event: dict, kind: str) -> List[dict]:
    content = event.get("message", {}).get("content", [])
    if not isinstance(content, list):
        return []
    return [b for b in content if isinstance(b, dict) and b.get("type") == kind]


def plugin_loaded(init: dict) -> bool:
    for key in ("plugins", "skills", "slash_commands", "mcp_servers", "agents"):
        if "manifestdocs" in json.dumps(init.get(key, [])).lower():
            return True
    return False


def harness_text(event: dict) -> str:
    """Text the harness, not a tool, put in the session: hook events and non-tool user blocks."""
    if event.get("type") != "user":
        return normalise(json.dumps(event))
    content = event.get("message", {}).get("content", [])
    if isinstance(content, str):
        return normalise(content)
    return normalise(json.dumps([b for b in content if b.get("type") != "tool_result"]))


def read_transcript(path: str):
    """Returns the init event, tool calls, tool results' text, harness text and result event."""
    init: dict = {}
    calls: List[dict] = []
    results: List[str] = []
    harness: List[str] = []
    final: dict = {}
    for event in events(path):
        kind = event.get("type")
        if kind == "system" and event.get("subtype") == "init":
            init = event
        elif kind == "assistant":
            calls += blocks(event, "tool_use")
        elif kind == "result":
            final = event
        else:
            harness.append(harness_text(event))
        if kind == "user":
            results += [normalise(result_text(b.get("content"))) for b in blocks(event, "tool_result")]
    return init, calls, results, harness, final


def docs_named(calls: List[dict]) -> List[str]:
    named = set()
    for call in calls:
        named.update(DOCS_PATH.findall(" " + json.dumps(call.get("input", {}))))
    return sorted(named)


def summarise(path: str, sentinels: List[str], injected: str) -> dict:
    init, calls, results, harness, result = read_transcript(path)
    seen_text = results + [normalise(injected)]
    commands = [c.get("input", {}).get("command", "") for c in calls if c.get("name") == "Bash"]
    usage = result.get("usage", {})
    return {
        "model": init.get("model"),
        "plugin_loaded": plugin_loaded(init),
        "outline_seen": any(OUTLINE in t for t in harness),
        "tool_calls": len(calls),
        "tools": dict(Counter(c.get("name") for c in calls)),
        "mfd_calls": sum(1 for c in commands if MFD_COMMAND.search(" " + c + " ")),
        "docs_opened": docs_named(calls),
        "facts_seen": [any(normalise(s) in t for t in seen_text) for s in sentinels],
        "turns": result.get("num_turns"),
        "duration_ms": result.get("duration_ms"),
        "cost_usd": result.get("total_cost_usd"),
        "input_tokens": usage.get("input_tokens"),
        "output_tokens": usage.get("output_tokens"),
        "cache_read_tokens": usage.get("cache_read_input_tokens"),
        "cache_write_tokens": usage.get("cache_creation_input_tokens"),
        "is_error": result.get("is_error", not result),
        "final_message": result.get("result"),
    }


def main(argv: Optional[List[str]] = None) -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--task-json", required=True)
    p.add_argument("--arm", required=True)
    p.add_argument("--transcript")
    p.add_argument("--injected")
    p.add_argument("--wall", type=float)
    args = p.parse_args(argv)

    with open(args.task_json) as f:
        task = json.load(f)
    sentinels = [fact["sentinel"] for fact in task["facts"]]
    injected = open(args.injected).read() if args.injected else ""

    out: Dict[str, object] = {
        "wall_s": args.wall,
        "transcript": args.transcript,
        "facts_injected": [normalise(s) in normalise(injected) for s in sentinels],
    }
    if args.transcript:
        out.update(summarise(args.transcript, sentinels, injected))
        out["needed_opened"] = [f for f in task["needed_files"] if f in out["docs_opened"]]
    json.dump(out, sys.stdout)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
