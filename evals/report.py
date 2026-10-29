"""Aggregates a results folder from run.sh into a Markdown table per hypothesis.

Usage: python3 report.py results/<run>

For each task and arm:
- solved: runs that passed every checklist item.
- checklist: the share of checklist items passed, as the mean over runs and the min-max range.
- hidden: the share of hidden-fact items passed, counted only in runs that passed every base
  item, as the mean over runs and the min-max range.
- facts seen: runs where every hidden fact's sentence reached the agent.
- calls, tokens, cost, wall: medians over runs. Tokens count input, output and cache reads
  and writes together.

The verdict compares each mfd arm with arm A on the hidden mean. A difference no larger than
the wider of the two arms' min-max ranges counts as no difference.
"""

import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path
from typing import Dict, List, Optional, Tuple


def load(folder: Path) -> Dict[Tuple[str, str], List[dict]]:
    groups: Dict[Tuple[str, str], List[dict]] = defaultdict(list)
    for path in sorted(folder.glob("*.json")):
        result = json.loads(path.read_text())
        groups[(result["task"], result["arm"])].append(result)
    return groups


def checklist_share(r: dict) -> float:
    total = len(r["passed"]) + len(r["failed"])
    return len(r["passed"]) / total if total else 0.0


def hidden_share(r: dict) -> float:
    # Hidden items count only when every base item passed. Some hidden items, such as H1's
    # "egg doesn't find eggplant", also pass on the untouched fixture, so an agent that did
    # nothing would otherwise score on them.
    hidden = set(r["hidden"])
    if set(r["failed"]) - hidden:
        return 0.0
    return len(hidden & set(r["passed"])) / len(hidden) if hidden else 1.0


def mean_range(values: List[float]) -> Tuple[str, float, float]:
    mean = statistics.mean(values)
    return f"{mean:.0%} ({min(values):.0%}–{max(values):.0%})", mean, max(values) - min(values)


def median(values: List[Optional[float]]) -> Optional[float]:
    known = [v for v in values if v is not None]
    return statistics.median(known) if known else None


def tokens(m: dict) -> Optional[int]:
    parts = [m.get(k) for k in ("input_tokens", "output_tokens", "cache_read_tokens", "cache_write_tokens")]
    return sum(p for p in parts if p) if any(parts) else None


def fmt(value: Optional[float], spec: str = ".0f") -> str:
    return "–" if value is None else format(value, spec)


def row(arm: str, runs: List[dict]) -> Tuple[str, float, float]:
    checklist, _, _ = mean_range([checklist_share(r) for r in runs])
    hidden, mean, spread = mean_range([hidden_share(r) for r in runs])
    metrics = [r.get("metrics", {}) for r in runs]
    solved = sum(1 for r in runs if not r["failed"])
    seen = sum(1 for m in metrics if m.get("facts_seen") and all(m["facts_seen"]))
    cells = [
        arm, f"{solved}/{len(runs)}", checklist, hidden,
        f"{seen}/{len(runs)}", fmt(median([m.get("tool_calls") for m in metrics])),
        fmt(median([tokens(m) for m in metrics])), fmt(median([m.get("cost_usd") for m in metrics]), ".2f"),
        fmt(median([m.get("wall_s") for m in metrics])),
    ]
    return "| " + " | ".join(cells) + " |", mean, spread


def verdict(arm: str, base: Tuple[float, float], other: Tuple[float, float]) -> str:
    diff = other[0] - base[0]
    noise = max(base[1], other[1])
    if abs(diff) <= noise:
        return f"{arm} vs A: no difference ({diff:+.0%}, spread {noise:.0%})"
    return f"{arm} vs A: {'better' if diff > 0 else 'worse'} by {diff:+.0%} (spread {noise:.0%})"


def task_section(task: str, arms: Dict[str, List[dict]]) -> List[str]:
    lines = [f"## {task}", "",
             "| Arm | Solved | Checklist (range) | Hidden (range) | Facts seen | Calls | Tokens | Cost $ | Wall s |",
             "|---|---|---|---|---|---|---|---|---|"]
    stats = {}
    for arm in sorted(arms):
        text, mean, spread = row(arm, arms[arm])
        lines.append(text)
        stats[arm] = (mean, spread)
    lines.append("")
    if "A" in stats:
        lines += [f"- {verdict(arm, stats['A'], stats[arm])}" for arm in sorted(stats) if arm != "A"]
    return lines + plugin_warnings(arms) + [""]


def plugin_warnings(arms: Dict[str, List[dict]]) -> List[str]:
    """Arm A must not see the plugin; arms B and C must load it and get its outline."""
    warnings = []
    for arm, runs in sorted(arms.items()):
        measured = [r["metrics"] for r in runs if "plugin_loaded" in r.get("metrics", {})]
        if arm == "A":
            bad = [m for m in measured if m["plugin_loaded"] or m["outline_seen"]]
            what = "saw the manifestdocs plugin"
        else:
            bad = [m for m in measured if not (m["plugin_loaded"] and m["outline_seen"])]
            what = "didn't load the plugin or get its session-start outline"
        if bad:
            warnings.append(f"- WARNING: {len(bad)} arm {arm} run(s) {what}")
    return warnings


def main(argv: List[str]) -> int:
    if len(argv) != 2:
        print(__doc__.strip().splitlines()[2], file=sys.stderr)
        return 2
    groups = load(Path(argv[1]))
    by_task: Dict[str, Dict[str, List[dict]]] = defaultdict(dict)
    for (task, arm), runs in groups.items():
        by_task[task][arm] = runs
    out = [f"# Pilot results: {Path(argv[1]).name}", ""]
    for task in sorted(by_task):
        out += task_section(task, by_task[task])
    print("\n".join(out))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
