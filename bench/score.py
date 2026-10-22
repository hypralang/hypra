#!/usr/bin/env python3
"""Scores a benchmark run: compiles every attempt and checks each task's expectations.

A run is a directory of solutions named `<task>.<attempt>.hypra` (attempt 1 is written before any compiler
feedback), or `<task>.hypra` for a single attempt.

usage: python3 bench/score.py bench/runs/<run> [--compiler target/release/hypra]
"""

import json
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

BENCH = Path(__file__).resolve().parent
TASKS = BENCH / "tasks"


def attempts(run: Path, task: str) -> list[Path]:
    numbered = sorted(run.glob(f"{task}.*.hypra"), key=lambda p: int(p.suffixes[-2][1:]))
    single = run / f"{task}.hypra"
    return numbered or ([single] if single.exists() else [])


def evaluate(compiler: str, solution: Path, task: Path) -> tuple[bool, list[str], list[str], list[str]]:
    """Returns (passed, error codes, warning codes, expectations not met)."""
    data = str(task / "data.json")
    check = subprocess.run(
        [compiler, "check", str(solution), "--data", data, "--format", "json"], capture_output=True, text=True
    )
    diagnostics = json.loads(check.stdout or "[]")
    errors = [d["code"] for d in diagnostics if d["severity"] == "error"]
    warnings = [d["code"] for d in diagnostics if d["severity"] == "warning"]
    if errors:
        return False, errors, warnings, []

    with tempfile.TemporaryDirectory() as out:
        subprocess.run([compiler, "build", str(solution), "--data", data, "--out", out], capture_output=True)
        html = (Path(out) / f"{solution.stem}.html").read_text()
    expected = [line for line in (task / "expect.txt").read_text().splitlines() if line]
    missing = [line for line in expected if not any(option in html for option in line.split("||"))]
    return not missing, [], warnings, missing


def main() -> None:
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    run = Path(args[0])
    compiler = args[args.index("--compiler") + 1] if "--compiler" in args else "target/release/hypra"

    rows, first_codes, first_warnings = [], Counter(), Counter()
    first_passes = final_passes = clean = 0
    for task in sorted(p for p in TASKS.iterdir() if p.is_dir()):
        tries = attempts(run, task.name)
        if not tries:
            rows.append(f"| {task.name} | – | – | | no solution |")
            continue
        results = [evaluate(compiler, t, task) for t in tries]
        first_ok, codes, warnings, _ = results[0]
        final_ok, final_codes, final_warnings, final_missing = results[-1]
        first_codes.update(codes)
        first_warnings.update(warnings)
        first_passes += first_ok
        final_passes += final_ok
        clean += first_ok and not warnings
        note = ", ".join(final_codes) or ", ".join(f"missing `{m}`" for m in final_missing)
        mark = lambda ok: "✔" if ok else "✘"
        warned = ", ".join(final_warnings)
        rows.append(f"| {task.name} | {mark(first_ok)} | {mark(final_ok)} ({len(tries)}) | {warned} | {note} |")

    total = len(rows)
    print(f"# Benchmark: {run.name}\n")
    print(f"- first try: **{first_passes}/{total}** ({first_passes * 100 // total}%)")
    print(f"- first try without warnings: **{clean}/{total}** ({clean * 100 // total}%)")
    print(f"- with compiler feedback: **{final_passes}/{total}** ({final_passes * 100 // total}%)\n")
    print("| task | first try | final (attempts) | warnings | problems |\n| --- | --- | --- | --- | --- |")
    print("\n".join(rows))
    for label, counter in [("Errors", first_codes), ("Warnings", first_warnings)]:
        if counter:
            print(f"\n{label} on the first try: " + ", ".join(f"`{c}` ×{n}" for c, n in counter.most_common()))


if __name__ == "__main__":
    main()
