//! [LIVE-PARENT-LIVENESS] Parent liveness must not launch child processes.

use std::{env, error::Error, io::Read as _, path::PathBuf, process::Command};

use super::process_is_alive;

type TestResult = Result<(), Box<dyn Error>>;

const DEAD_PID_ENV: &str = "DESLOP_PROBE_DEAD_PID";
const PYTHON_ENV: &str = "DESLOP_TEST_PYTHON";
const DRIVER_ENV: &str = "DESLOP_WINDOWS_JOB_DRIVER";
const PYTHON: &str = "python";
const DRIVER: &str = "../../scripts/lib/windows_job_probe.py";
const TEST_NAME: &str = "process::tests::parent_probe_uses_no_child_processes";
const START_SIGNAL: u8 = b'\n';
const SYSTEM_IDLE_PID: u32 = 0;
const INVALID_PID: u32 = u32::MAX;

/// Python assigns this isolated process to its job before releasing stdin.
fn wait_for_job_assignment() -> TestResult {
    let mut signal = [u8::default()];
    std::io::stdin().read_exact(&mut signal)?;
    assert_eq!(signal, [START_SIGNAL]);
    Ok(())
}

fn probe_after_assignment(dead_pid: u32) -> TestResult {
    wait_for_job_assignment()?;
    assert!(
        process_is_alive(std::process::id()),
        "the running probe is alive"
    );
    assert!(
        !process_is_alive(dead_pid),
        "the reaped process is no longer alive"
    );
    assert!(
        !process_is_alive(SYSTEM_IDLE_PID),
        "PID zero cannot be a parent"
    );
    assert!(!process_is_alive(INVALID_PID), "invalid PID is dead");
    Ok(())
}

/// Keeps the normal suite portable while allowing the same source to run in Wine.
fn driver_path() -> PathBuf {
    env::var_os(DRIVER_ENV).map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DRIVER),
        PathBuf::from,
    )
}

#[cfg(windows)]
#[test]
fn parent_probe_uses_no_child_processes() -> TestResult {
    if let Ok(dead_pid) = env::var(DEAD_PID_ENV) {
        return probe_after_assignment(dead_pid.parse()?);
    }
    let python = env::var_os(PYTHON_ENV).unwrap_or_else(|| PYTHON.into());
    let output = Command::new(python)
        .arg(driver_path())
        .arg(env::current_exe()?)
        .arg(TEST_NAME)
        .output()?;
    assert!(
        output.status.success(),
        "job-accounted probe failed: {output:?}"
    );
    Ok(())
}
