#!/usr/bin/env python3
"""Measure what one saved edit costs a running LSP, emitting Markdown.

Owns a real `deslop-lsp` on a workspace, waits for the cold pass, then appends
one new function to one source file per round and waits for the report that
analysed it ([LIVE-PERF-BUDGETS]). Every figure is read from the process (CPU
seconds, resident memory) or from the stage records the server logs; nothing is
typed. Run it once per binary with the same `--report` to compare builds.

Usage: python3 scripts/reports/edit_cost.py --workspace COPY --file src/a.rs \
    --binary target/release/deslop-lsp --label head
"""
import argparse
import hashlib
import json
from pathlib import Path
import queue
import subprocess
import threading
import time

from idle_churn import ROOT, boot, receive, request, wait_for, is_idle
from idle_cpu import cpu_seconds

ARTIFACTS = ROOT / "target/edit-cost"
DEFAULT_REPORT = ROOT / "reports/live-edit-cost.md"
DEFAULT_ROUNDS = 3
DEFAULT_APPEND = "\npub fn edit_cost_probe_{round}() -> usize {{\n    {round}\n}}\n"
PASS_TIMEOUT_SECONDS = 3600
RSS_INTERVAL_SECONDS = 0.1
KIB_PER_MIB = 1024
STAGE_RECORD = "cluster stage complete"
REPORT_GET = "deslop/reportGet"
FIRST_REQUEST_ID = 10
MILLIS_PER_SECOND = 1000


def resident_mib(pid):
    """Resident memory of the server as the kernel reports it."""
    out = subprocess.run(["ps", "-p", str(pid), "-o", "rss="], capture_output=True, text=True)
    return int(out.stdout.strip() or 0) / KIB_PER_MIB


def watch_memory(pid, stop, peak):
    """Track the highest resident memory seen until stopped."""
    while not stop.wait(RSS_INTERVAL_SECONDS):
        peak["mib"] = max(peak["mib"], resident_mib(pid))


def fields(line):
    """The `key=value` fields of one structured log record."""
    pairs = (token.split("=", 1) for token in line.split() if "=" in token)
    return {key: value.strip('"') for key, value in pairs}


def stages_since(log_path, offset):
    """Stage records the server wrote after `offset`, and the new offset."""
    with log_path.open("rb") as log:
        log.seek(offset)
        text = log.read()
    records = [fields(line) for line in text.decode(errors="replace").splitlines()
               if STAGE_RECORD in line]
    return records, offset + len(text)


def measured(child, work):
    """Run `work` and return its result with wall, CPU and peak memory."""
    stop, peak = threading.Event(), {"mib": resident_mib(child.pid)}
    watcher = threading.Thread(target=watch_memory, args=(child.pid, stop, peak))
    cpu, wall = cpu_seconds(child.pid), time.monotonic()
    watcher.start()
    try:
        result = work()
    finally:
        stop.set()
        watcher.join()
    return result, {"wall_s": time.monotonic() - wall,
                    "cpu_s": cpu_seconds(child.pid) - cpu, "peak_rss_mib": peak["mib"]}


def await_new_report(session, previous, request_id):
    """Wait for idle passes until the server holds a report other than `previous`."""
    child, messages, observed = session
    while True:
        wait_for(messages, is_idle, observed, PASS_TIMEOUT_SECONDS)
        report = request(child, messages, REPORT_GET, request_id, observed, PASS_TIMEOUT_SECONDS)
        if report != previous:
            return report
        request_id += 1


def edit(session, args, round_index, previous):
    """Append one function to the file and wait for the report that saw it."""
    target = args.workspace / args.file
    with target.open("a") as source:
        source.write(args.append.format(round=round_index))
    return await_new_report(session, previous, FIRST_REQUEST_ID * (round_index + 1))


def stage_summary(records):
    """Total stage time, and what candidate pairing was fed."""
    pairing = [record for record in records if record.get("stage") == "candidate_pairs"]
    total = sum(int(record["elapsed_ms"]) for record in records)
    return {"stage_s": total / MILLIS_PER_SECOND,
            "pairing_s": sum(int(r["elapsed_ms"]) for r in pairing) / MILLIS_PER_SECOND,
            "pairing_input": max((int(r["input"]) for r in pairing), default=0)}


def run_rounds(session, args, log_path):
    """Measure the cold pass and then each edit."""
    child, messages, observed = session
    cold = lambda: boot(child, messages, observed, args.workspace, PASS_TIMEOUT_SECONDS)
    report, cost = measured(child, cold)
    records, offset = stages_since(log_path, 0)
    rows = [{"pass": "cold", **cost, **stage_summary(records)}]
    for round_index in range(1, args.rounds + 1):
        work = lambda: edit(session, args, round_index, report)
        report, cost = measured(child, work)
        records, offset = stages_since(log_path, offset)
        rows.append({"pass": f"edit {round_index}", **cost, **stage_summary(records)})
    rows.append({"pass": "idle after", "wall_s": 0.0, "cpu_s": 0.0,
                 "peak_rss_mib": resident_mib(child.pid), **stage_summary([])})
    return rows, report


def binary_digest(path):
    """Identify the measured executable."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def files_analysed(report):
    """The corpus size the final report states."""
    return report.get("files_analysed", report.get("metrics", {}).get("files_analysed"))


def write_report(args, rows, report):
    """Append this binary's table to the Markdown report."""
    header = "| Pass | Wall (s) | CPU (s) | Peak RSS (MiB) | Stage time (s) | Pairing (s) | Pairing input |"
    lines = ["", f"## {args.label}", "", "Generated by `scripts/reports/edit_cost.py`.", "",
             f"- Binary SHA-256: `{binary_digest(args.binary)}`",
             f"- Files analysed: {files_analysed(report)}",
             f"- Edited file: one function appended per round, {args.rounds} rounds",
             "- CPU is process CPU seconds, so it does not depend on machine load.", "",
             header, "|---|---|---|---|---|---|---|"]
    lines += [f"| {row['pass']} | {row['wall_s']:.1f} | {row['cpu_s']:.1f} | "
              f"{row['peak_rss_mib']:.0f} | {row['stage_s']:.1f} | {row['pairing_s']:.1f} | "
              f"{row['pairing_input']} |" for row in rows]
    with args.report.open("a") as out:
        out.write("\n".join(lines) + "\n")
    (ARTIFACTS / f"{args.label}.json").write_text(json.dumps(rows, indent=2) + "\n")


def arguments():
    """Choose the binary, the workspace copy and the file to edit."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/deslop-lsp")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--file", required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument("--rounds", type=int, default=DEFAULT_ROUNDS)
    parser.add_argument("--append", default=DEFAULT_APPEND)
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT)
    return parser.parse_args()


def main():
    """Own the child, measure, and close its transport."""
    args = arguments()
    args.workspace = args.workspace.resolve()
    ARTIFACTS.mkdir(parents=True, exist_ok=True)
    log_path = ARTIFACTS / f"{args.label}-stderr.log"
    with log_path.open("wb") as log:
        child = subprocess.Popen([str(args.binary), str(args.workspace), "--ipc-transport", "tcp"],
                                 stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=log)
        messages, observed = queue.Queue(), []
        reader = threading.Thread(target=receive, args=(child.stdout, messages), daemon=True)
        reader.start()
        try:
            rows, report = run_rounds((child, messages, observed), args, log_path)
        finally:
            child.stdin.close()
            child.wait(timeout=PASS_TIMEOUT_SECONDS)
            reader.join()
    write_report(args, rows, report)
    print(args.report)


if __name__ == "__main__":
    main()
