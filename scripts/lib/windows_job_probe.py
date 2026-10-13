"""Assert real Windows liveness probes create no children ([LIVE-PARENT-LIVENESS])."""

import ctypes
from ctypes import wintypes
import json
import os
import subprocess
import sys

DEAD_PID_ENV = "DESLOP_PROBE_DEAD_PID"
EXPECTED_JOB_PROCESSES = 1
JOB_BASIC_ACCOUNTING = 1
PROCESS_ASSIGN_ACCESS = 0x0101  # PROCESS_SET_QUOTA | PROCESS_TERMINATE
FAILURE_TIMEOUT_SECONDS = 30
START_SIGNAL = b"\n"
TEST_FLAGS = ("--exact", "--nocapture")
DEAD_PROCESS_SCRIPT = "pass"


class Accounting(ctypes.Structure):
    """Win32 JOBOBJECT_BASIC_ACCOUNTING_INFORMATION layout."""

    _fields_ = [(name, ctypes.c_int64) for name in (
        "user_time", "kernel_time", "period_user_time", "period_kernel_time")]
    _fields_ += [(name, wintypes.DWORD) for name in (
        "page_faults", "total_processes", "active_processes", "terminated_processes")]


def kernel_api():
    """Declare pointer widths explicitly before calling Win32."""
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    signatures = {
        "CreateJobObjectW": (wintypes.HANDLE, (ctypes.c_void_p, wintypes.LPCWSTR)),
        "OpenProcess": (wintypes.HANDLE, (wintypes.DWORD, wintypes.BOOL, wintypes.DWORD)),
        "AssignProcessToJobObject": (wintypes.BOOL, (wintypes.HANDLE, wintypes.HANDLE)),
        "QueryInformationJobObject": (wintypes.BOOL, (
            wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD, ctypes.c_void_p)),
        "CloseHandle": (wintypes.BOOL, (wintypes.HANDLE,)),
    }
    for name, (result, arguments) in signatures.items():
        function = getattr(kernel, name)
        function.restype, function.argtypes = result, arguments
    return kernel


def checked(value):
    """Preserve the native error rather than treating a failed probe as success."""
    if not value:
        raise ctypes.WinError(ctypes.get_last_error())
    return value


def assign(kernel, job, pid):
    """Assign the waiting probe before it can run any liveness checks."""
    process = checked(kernel.OpenProcess(PROCESS_ASSIGN_ACCESS, False, pid))
    try:
        checked(kernel.AssignProcessToJobObject(job, process))
    finally:
        checked(kernel.CloseHandle(process))


def run_probe(kernel, job, dead_pid, binary, test_name):
    """A pipe release synchronizes assignment; the timeout only detects hangs."""
    environment = dict(os.environ, **{DEAD_PID_ENV: str(dead_pid)})
    command = [binary, TEST_FLAGS[0], test_name, TEST_FLAGS[1]]
    with subprocess.Popen(command, env=environment, stdin=subprocess.PIPE,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE) as process:
        try:
            assign(kernel, job, process.pid)
            stdout, stderr = process.communicate(START_SIGNAL, timeout=FAILURE_TIMEOUT_SECONDS)
            if process.returncode:
                raise AssertionError(f"Liveness controls failed: {stdout!r} {stderr!r}")
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()


def count_job_processes(kernel, job):
    """Cumulative accounting retains exited children, eliminating sampling races."""
    accounting = Accounting()
    checked(kernel.QueryInformationJobObject(
        job, JOB_BASIC_ACCOUNTING, ctypes.byref(accounting), ctypes.sizeof(accounting), None))
    return accounting.total_processes


def main():
    """Retain the reaped child's handle so its PID cannot be recycled during checks."""
    kernel = kernel_api()
    job = checked(kernel.CreateJobObjectW(None, None))
    try:
        with subprocess.Popen([sys.executable, "-c", DEAD_PROCESS_SCRIPT]) as dead:
            if dead.wait(timeout=FAILURE_TIMEOUT_SECONDS):
                raise AssertionError("The controlled process did not exit successfully")
            run_probe(kernel, job, dead.pid, *sys.argv[1:])
            total = count_job_processes(kernel, job)
            print(json.dumps({"live_and_dead_controls": "passed", "job_processes": total}))
            if total != EXPECTED_JOB_PROCESSES:
                raise AssertionError(f"Liveness checks launched children: expected 1 process, got {total}")
    finally:
        checked(kernel.CloseHandle(job))


if __name__ == "__main__":
    main()
