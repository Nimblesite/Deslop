#!/usr/bin/env python3
"""Measure a completed LSP before, during, and after excluded-file churn.

`--churn deletion` writes and deletes unsupported files; `--churn scoping`
writes and deletes the corpus-scoping files ([LIVE-CONFIG-LIVE]: `osprey.toml`,
`.gitignore`, `.ignore`) inside the excluded tree, the git-worktree pattern that
kept the Osprey window re-analysing.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_BINARY = ROOT / "target/release/deslop-lsp"
ARTIFACTS = ROOT / "target/idle-cpu-churn"
DEFAULT_REPORT = ROOT / "reports/idle-cpu-excluded-churn.md"
SCOPING_FILES = ("osprey.toml", ".gitignore", ".ignore")
SCOPING_SOURCE = b"# scratch scoping file\n"
SOURCE = b"build output\n"
BATCH_SIZE = 128
BATCH_INTERVAL_SECONDS = 0.025
MEASUREMENT_SECONDS = 8
RESPONSE_TIMEOUT_SECONDS = 180


def receive(stream, messages):
    """Decode actual LSP stdout frames into the conversation queue."""
    while header := stream.readline():
        headers = {}
        while header.strip():
            name, value = header.decode().split(":", 1)
            headers[name.lower()] = value.strip()
            header = stream.readline()
        message = json.loads(stream.read(int(headers["content-length"])))
        messages.put(message)


def send(stream, method, params, request_id=None):
    """Write one framed JSON-RPC message to the running binary."""
    message = {"jsonrpc": "2.0", "method": method, "params": params}
    if request_id is not None:
        message["id"] = request_id
    body = json.dumps(message).encode()
    stream.write(f"Content-Length: {len(body)}\r\n\r\n".encode() + body)
    stream.flush()


def wait_for(messages, predicate, observed):
    """Consume notifications until the requested response or state arrives."""
    deadline = time.monotonic() + RESPONSE_TIMEOUT_SECONDS
    while True:
        message = messages.get(timeout=max(deadline - time.monotonic(), 0))
        observed.append(message)
        if predicate(message):
            return message


def request(child, messages, method, request_id, observed):
    """Issue a request and require a successful server response."""
    send(child.stdin, method, {}, request_id)
    response = wait_for(messages, lambda item: item.get("id") == request_id, observed)
    if "error" in response:
        raise RuntimeError(response["error"])
    return response["result"]


def boot(child, messages, observed):
    """Wait for the real startup terminal idle notification."""
    params = {"processId": os.getpid(), "rootUri": ROOT.as_uri(), "capabilities": {}}
    send(child.stdin, "initialize", params, 1)
    wait_for(messages, lambda item: item.get("id") == 1, observed)
    send(child.stdin, "initialized", {})
    wait_for(messages, lambda item: item.get("method") == "deslop/analysisState"
             and item.get("params", {}).get("state") == "idle", observed)
    return request(child, messages, "deslop/reportGet", 2, observed)


def deletion_batch(directory, batch):
    """Unique, unsupported files inside excluded target/."""
    return [(directory / f"temporary-{batch}-{index}.tmp", SOURCE) for index in range(BATCH_SIZE)]


def scoping_batch(directory, batch):
    """Corpus-scoping files inside a fresh excluded subdirectory."""
    nested = directory / f"worktree-{batch}"
    nested.mkdir(exist_ok=True)
    return [(nested / name, SCOPING_SOURCE) for name in SCOPING_FILES]


BATCHES = {"deletion": deletion_batch, "scoping": scoping_batch}


def churn(stop, result, kind):
    """Create and delete files inside excluded target/ until stopped."""
    directory = ARTIFACTS / "build-output"
    directory.mkdir(exist_ok=True)
    result.update(removed_files=0, batches=0, churn=kind)
    while not stop.is_set():
        paths = BATCHES[kind](directory, result["batches"])
        for path, body in paths:
            path.write_bytes(body)
        for path, _ in paths:
            path.unlink()
        result["removed_files"] += len(paths)
        result["batches"] += 1
        stop.wait(BATCH_INTERVAL_SECONDS)


def measure(child, label, args):
    """Reuse the native sampler, which validates executable identity."""
    command = [sys.executable, str(ROOT / "scripts/reports/idle_cpu.py"),
               "--pid", str(child.pid), "--label", label, "--binary", str(args.binary),
               "--report", str(args.report), "--artifacts", str(ARTIFACTS),
               "--duration", str(MEASUREMENT_SECONDS)]
    subprocess.run(command, check=True)
    return json.loads((ARTIFACTS / f"{label}.json").read_text())


def digest(report):
    """Compare the complete rendered wire report without text matching."""
    return hashlib.sha256(json.dumps(report, sort_keys=True).encode()).hexdigest()


def record_workload(before, after, observed, result, args):
    """Emit workload facts and unchanged-report evidence mechanically."""
    label = args.label_prefix
    states = [item["params"] for item in observed
              if item.get("method") == "deslop/analysisState"]
    reruns = sum(1 for state in states[1:] if state.get("state") != "idle")
    result.update({"before_digest": digest(before), "after_digest": digest(after),
                   "unchanged_report": before == after, "analysis_states": states,
                   "post_boot_analysis_states": reruns})
    (ARTIFACTS / f"{label}-workload.json").write_text(json.dumps(result, indent=2) + "\n")
    for phase, snapshot in (("before", before), ("after", after)):
        (ARTIFACTS / f"{label}-{phase}-report.json").write_text(json.dumps(snapshot) + "\n")
    lines = ["## Controlled workload", "", "Generated by `scripts/reports/idle_churn.py`.", "",
             f"- Binary: `{args.binary}`", f"- Churn kind: {result['churn']}",
             f"- Removed excluded files: {result['removed_files']} in {result['batches']} batches",
             f"- Unchanged complete wire report: {result['unchanged_report']}",
             f"- Observed analysis states: {len(states)}",
             f"- Non-idle analysis states after boot: {reruns}",
             "- Churn generated only under `target/idle-cpu-churn/build-output/`.",
             "- Measurements cover active external build churn, not an untouched workspace."]
    with args.report.open("a") as report:
        report.write("\n".join(lines) + "\n")


def run_measurements(child, messages, observed, args):
    """Measure control, then bounded churn, then quiet recovery."""
    label = args.label_prefix
    before = boot(child, messages, observed)
    measure(child, f"{label}-before", args)
    stop, result = threading.Event(), {}
    worker = threading.Thread(target=churn, args=(stop, result, args.churn))
    worker.start()
    try:
        measure(child, f"{label}-active", args)
    finally:
        stop.set()
        worker.join()
    measure(child, f"{label}-after", args)
    after = request(child, messages, "deslop/reportGet", 3, observed)
    record_workload(before, after, observed, result, args)


def arguments():
    """Choose the binary, churn kind, report and evidence label."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--label-prefix", default=f"churn-{time.time_ns()}")
    parser.add_argument("--binary", type=Path, default=DEFAULT_BINARY)
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT)
    parser.add_argument("--churn", choices=sorted(BATCHES), default="deletion")
    return parser.parse_args()


def main():
    """Own the child and close its transport after the measurement."""
    args = arguments()
    ARTIFACTS.mkdir(parents=True, exist_ok=True)
    with (ARTIFACTS / "lsp-stderr.log").open("wb") as log:
        child = subprocess.Popen([str(args.binary), str(ROOT), "--ipc-transport", "tcp"],
                                 stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=log)
        messages, observed = queue.Queue(), []
        reader = threading.Thread(target=receive, args=(child.stdout, messages), daemon=True)
        reader.start()
        try:
            run_measurements(child, messages, observed, args)
        finally:
            child.stdin.close()
            child.wait(timeout=RESPONSE_TIMEOUT_SECONDS)
            reader.join()
    print(args.report)


if __name__ == "__main__":
    main()
