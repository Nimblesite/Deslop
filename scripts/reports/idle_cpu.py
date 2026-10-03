#!/usr/bin/env python3
"""Measure one running Deslop process with macOS native sampling, emitting Markdown.

Usage: python3 scripts/reports/idle_cpu.py --pid PID --label idle \
    --binary target/release/deslop-lsp --report reports/idle-cpu-native-sampling.md
"""
import argparse
import ctypes
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import socket
import subprocess
import time

SECONDS_PER_MINUTE = 60
PERCENT_SCALE = 100
DEFAULT_DURATION = 5
SAMPLE_INTERVAL_MS = 10
SOCKET_TIMEOUT = 10
REPORT_TITLE = "# Native LSP idle CPU measurement"
GENERATOR = "scripts/reports/idle_cpu.py"
PROCESS_PATH_CAPACITY = 4096
PROCESS_START_FORMAT = "%a %b %d %H:%M:%S %Y"
NANOSECONDS_PER_SECOND = 1_000_000_000


def cpu_seconds(pid):
    """Read cumulative process CPU time at macOS ps centisecond resolution."""
    result = subprocess.check_output(["ps", "-p", str(pid), "-o", "time="], text=True).strip()
    total = 0.0
    for component in result.split(":"):
        total = total * SECONDS_PER_MINUTE + float(component)
    return total


def process_executable(pid):
    """Resolve the executable from the kernel instead of trusting supplied argv."""
    library = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
    library.proc_pidpath.argtypes = [ctypes.c_int, ctypes.c_void_p, ctypes.c_uint32]
    library.proc_pidpath.restype = ctypes.c_int
    buffer = ctypes.create_string_buffer(PROCESS_PATH_CAPACITY)
    if library.proc_pidpath(pid, buffer, len(buffer)) <= 0:
        raise OSError(ctypes.get_errno(), "Unable to read process executable")
    return Path(os.fsdecode(buffer.value)).resolve(strict=True)


def process_started(pid):
    """Read launch time with a fixed locale; ps resolves it to whole seconds."""
    environment = dict(os.environ, LC_ALL="C", TZ="UTC")
    text = subprocess.check_output(["ps", "-p", str(pid), "-o", "lstart="],
                                   env=environment, text=True).strip()
    return datetime.datetime.strptime(text, PROCESS_START_FORMAT).replace(tzinfo=datetime.timezone.utc)


def file_identity(path):
    """Capture metadata that changes if the executable is replaced or rewritten."""
    stat = path.stat()
    return {"device": stat.st_dev, "inode": stat.st_ino, "size": stat.st_size,
            "modified_ns": stat.st_mtime_ns, "changed_ns": stat.st_ctime_ns}


def verified_identity(args):
    """Bind supplied bytes to a live process whose launch postdates those bytes."""
    executable = process_executable(args.pid)
    if executable != args.binary.resolve(strict=True):
        raise ValueError(f"PID {args.pid} runs {executable}, not the supplied binary")
    started = process_started(args.pid)
    identity = file_identity(executable)
    latest_change = max(identity["modified_ns"], identity["changed_ns"])
    if latest_change >= int(started.timestamp()) * NANOSECONDS_PER_SECOND:
        raise ValueError("Binary changed in or after the launch second; provenance is unverifiable")
    return {"executable": str(executable), "started_utc": started.isoformat(), **identity}


def disconnect_subscriber(endpoint_path):
    """Subscribe to report notifications, verify the acknowledgement, then disconnect."""
    endpoint = json.loads(endpoint_path.read_text())
    request = {"jsonrpc": "2.0", "id": 1, "method": "report/subscribe", "params": {}}
    with socket.create_connection(("127.0.0.1", endpoint["port"]), SOCKET_TIMEOUT) as stream:
        stream.sendall((endpoint["token"] + "\n" + json.dumps(request) + "\n").encode())
        with stream.makefile("rb") as reader:
            acknowledgement = json.loads(reader.readline())
            if not acknowledgement.get("result", {}).get("subscribed"):
                raise RuntimeError(f"Subscription failed: {acknowledgement}")
    return acknowledgement["result"]["generation"]


def measure(args):
    """Sample stacks while measuring CPU-time growth over the same wall-clock window."""
    identity = verified_identity(args)
    digest = hashlib.sha256(args.binary.read_bytes()).hexdigest()
    before = cpu_seconds(args.pid)
    started = time.monotonic()
    output = args.artifacts / f"{args.label}.sample.txt"
    subprocess.run(["sample", str(args.pid), str(args.duration), str(SAMPLE_INTERVAL_MS),
                    "-file", str(output)], check=True, capture_output=True)
    after = cpu_seconds(args.pid)
    elapsed = time.monotonic() - started
    if verified_identity(args) != identity:
        raise RuntimeError("Process or binary identity changed during sampling; measurement rejected")
    return {"cpu_before": before, "cpu_after": after, "wall_seconds": elapsed,
            "cpu_seconds": after - before, "average_cpu_percent":
            (after - before) / elapsed * PERCENT_SCALE, "sample_file": str(output.resolve()),
            "process_identity": identity, "binary_sha256": digest}


def report_text(args, result, generation):
    """Render only values collected by this measurement."""
    identity = result["process_identity"]
    lines = [f"## {args.label}", "", f"- Measured: {datetime.datetime.now().astimezone().isoformat()}",
             f"- PID: {args.pid}", f"- Kernel executable path: `{identity['executable']}`",
             f"- Process launch (UTC, whole-second resolution): {identity['started_utc']}",
             f"- SHA-256: `{result['binary_sha256']}`",
             "- Provenance verified: executable path matches; file timestamps predate launch; process and file identity unchanged across sample.",
             f"- Wall-clock measurement window: {result['wall_seconds']:.3f} s",
             f"- Process CPU delta: {result['cpu_seconds']:.2f} s",
             f"- Average CPU: {result['average_cpu_percent']:.3f}% of one CPU core",
             f"- Native stack sample: `{result['sample_file']}`",
             "- Resolution: macOS `ps` cumulative CPU time rounds to centiseconds.",
             f"- Host: {platform.system()} {platform.release()} {platform.machine()}",
             "- Workload state: described by the caller's label; idle and analysis completion are not verified by this sampler.",
             "- Limitation: this observation does not rule out workload-specific or Windows idle CPU defects."]
    if generation is not None:
        lines.append(f"- Disconnected TCP subscriber acknowledged generation: {generation}")
    return "\n".join(lines) + "\n"


def arguments():
    """Read measurement options without changing the measured process."""
    parser = argparse.ArgumentParser(description=__doc__)
    for option, kind in (("pid", int), ("label", str), ("binary", Path), ("report", Path)):
        parser.add_argument(f"--{option}", type=kind, required=True)
    parser.add_argument("--duration", type=int, default=DEFAULT_DURATION)
    parser.add_argument("--artifacts", type=Path, default=Path("target/idle-cpu-measurement"))
    parser.add_argument("--disconnect-subscriber", type=Path)
    return parser.parse_args()


def save_measurement(args, result, generation):
    """Keep raw evidence alongside the generated report."""
    evidence = {"label": args.label, "pid": args.pid, "generation": generation, **result}
    (args.artifacts / f"{args.label}.json").write_text(json.dumps(evidence, indent=2) + "\n")
    args.report.parent.mkdir(parents=True, exist_ok=True)
    if not args.report.exists():
        args.report.write_text(REPORT_TITLE + f"\n\nGenerated by `{GENERATOR}`.\n\n")
    with args.report.open("a") as report:
        report.write(report_text(args, result, generation) + "\n")


def main():
    """Measure once and append the result; never continually poll the target."""
    args = arguments()
    args.artifacts.mkdir(parents=True, exist_ok=True)
    generation = disconnect_subscriber(args.disconnect_subscriber) if args.disconnect_subscriber else None
    result = measure(args)
    save_measurement(args, result, generation)
    print(json.dumps(result))


if __name__ == "__main__":
    main()
