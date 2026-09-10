#!/usr/bin/env python3
"""Read the running engine's current duplication metrics over its IPC socket.

The figures a dedup report quotes must come from the engine that measured
them, not from a number an agent typed. This asks the live LSP the same
`report/get` question the MCP surface asks, and writes the `metrics`
object it answers with, verbatim, to a JSON file the report generator
then reads.

Usage:
  live_metrics.py <workspace-root> <out.json>
"""
import json
import socket
import sys
from pathlib import Path

SOCKET_PATH = ".deslop/cache/deslop.sock"
REPORT_GET = "report/get"
REQUEST_ID = 1
CHUNK_BYTES = 1 << 16


def fetch_report(socket_path):
    """One `report/get` round trip over the newline-delimited JSON-RPC socket."""
    request = json.dumps(
        {"jsonrpc": "2.0", "id": REQUEST_ID, "method": REPORT_GET, "params": {}}
    ).encode()
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as stream:
        stream.connect(str(socket_path))
        stream.sendall(request + b"\n")
        buffered = b""
        while b"\n" not in buffered:
            chunk = stream.recv(CHUNK_BYTES)
            if not chunk:
                raise SystemExit("engine closed the socket before answering")
            buffered += chunk
    return json.loads(buffered.split(b"\n", 1)[0])


def metrics_of(response):
    """The metrics object, wherever the reply envelope carries it."""
    result = response.get("result", response)
    for candidate in (result, result.get("report", {})):
        if isinstance(candidate, dict) and "metrics" in candidate:
            return candidate["metrics"]
    raise SystemExit(f"no metrics in reply: {json.dumps(response)[:400]}")


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    root, out = Path(sys.argv[1]), Path(sys.argv[2])
    metrics = metrics_of(fetch_report(root / SOCKET_PATH))
    out.write_text(json.dumps({"metrics": metrics}, indent=2), encoding="utf-8")
    print(
        f"{metrics['duplicated_loc']} duplicated LOC, "
        f"{metrics['clusters_total']} clusters, "
        f"{metrics['duplication_percent']:.4f}%"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
