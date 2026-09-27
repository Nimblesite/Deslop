//! [CORPUS-MEASURE-PEAK-CPU] The busiest stretch of a scan, as a CPU rate.
//!
//! No kernel keeps a peak CPU figure the way it keeps a peak working set, so
//! this one is sampled: every [`CPU_SAMPLE_WINDOW`] the sampler reads the total
//! CPU time the scan's process tree has consumed, and the peak is the largest
//! rise over any one window, divided by that window's wall time.
//!
//! One implementation for every platform — `sysinfo` reads the same counter
//! `/usr/bin/time` and `TotalProcessorTime` report — so a peak measured on
//! Windows and one measured on macOS mean the same thing.
//!
//! The tree, not the pid: on POSIX the spawned process is `/usr/bin/time` and
//! the scan is its child, so the sampler sums every process descended from the
//! one it was given. A process that exits takes its CPU time out of the sum;
//! the fall that leaves is not a negative rate, and reads as no work.

use std::{
    sync::mpsc::{self, RecvTimeoutError, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// How often the sampler reads the tree's CPU time, and so the interval a peak
/// is averaged over. Half a second is short enough to catch a parallel phase
/// of a scan and long enough that one scheduler tick cannot dominate it.
pub const CPU_SAMPLE_WINDOW: Duration = Duration::from_millis(500);

/// How far up a parent chain the sampler walks before giving up. A reused pid
/// can make a chain loop; a real scan is never nested this deep.
const MAX_ANCESTRY_DEPTH: usize = 64;
/// Percentage base, named so the calculation reads as one.
const PERCENT: f64 = 100.0;

/// The tree's total CPU time at one moment of the scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuSample {
    /// Wall time since sampling began.
    pub at: Duration,
    /// CPU milliseconds the tree had consumed by then.
    pub cpu_ms: u64,
}

/// The highest CPU rate between any two consecutive samples, in percent of
/// one core — `250` is two and a half cores busy for that whole window.
///
/// Absent with fewer than two samples: a process that ended inside its first
/// window leaves no interval to measure a rate over, and an absent figure is
/// printed as absent, never as an idle scan.
#[must_use]
pub fn peak_cpu_percent(samples: &[CpuSample]) -> Option<f64> {
    samples
        .windows(2)
        .filter_map(|pair| match pair {
            [before, after] => window_percent(*before, *after),
            _ => None,
        })
        .reduce(f64::max)
}

/// The CPU rate across one window, or nothing when no wall time passed.
fn window_percent(before: CpuSample, after: CpuSample) -> Option<f64> {
    let wall = after.at.checked_sub(before.at)?.as_secs_f64();
    let cpu_ms = after.cpu_ms.saturating_sub(before.cpu_ms);
    (wall > 0.0).then(|| PERCENT * as_seconds(cpu_ms) / wall)
}

/// Milliseconds as seconds, keeping the lossy cast in one reviewed place.
fn as_seconds(milliseconds: u64) -> f64 {
    Duration::from_millis(milliseconds).as_secs_f64()
}

/// A running sampler, stopped by [`Sampler::finish`].
#[derive(Debug)]
pub(super) struct Sampler {
    /// Dropping or sending on this ends the sampling loop at once.
    stop: Sender<()>,
    /// The sampling thread, which hands back everything it read.
    worker: JoinHandle<Vec<CpuSample>>,
}

impl Sampler {
    /// Starts sampling the process tree headed by `root`.
    pub(super) fn start(root: u32) -> Self {
        let (stop, stopped) = mpsc::channel();
        let worker = thread::spawn(move || sample_until_stopped(Pid::from_u32(root), &stopped));
        Self { stop, worker }
    }

    /// Stops sampling and returns every sample taken.
    ///
    /// # Errors
    ///
    /// Returns an error when the sampling thread panicked.
    pub(super) fn finish(self) -> Result<Vec<CpuSample>> {
        // A send fails only when the worker has already returned, which is the
        // outcome this asks for.
        let _stopped = self.stop.send(());
        self.worker
            .join()
            .map_err(|_| anyhow!("the CPU sampler thread panicked"))
    }
}

/// Reads the tree's CPU time once per window until told to stop. Waiting on
/// the stop channel *is* the pacing, so a finished scan ends sampling at once.
fn sample_until_stopped(root: Pid, stopped: &mpsc::Receiver<()>) -> Vec<CpuSample> {
    let mut system = System::new();
    let mut samples = Vec::new();
    let started = Instant::now();
    loop {
        samples.extend(take_sample(&mut system, root, started));
        if !matches!(
            stopped.recv_timeout(CPU_SAMPLE_WINDOW),
            Err(RecvTimeoutError::Timeout)
        ) {
            return samples;
        }
    }
}

/// One reading of the tree's CPU time, or nothing once `root` is gone.
fn take_sample(system: &mut System, root: Pid, started: Instant) -> Option<CpuSample> {
    let refresh = ProcessRefreshKind::nothing().with_cpu().without_tasks();
    let _updated = system.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh);
    tree_cpu_ms(system, root).map(|cpu_ms| CpuSample {
        at: started.elapsed(),
        cpu_ms,
    })
}

/// CPU milliseconds consumed by `root` and every process descended from it,
/// or nothing once `root` itself is gone.
fn tree_cpu_ms(system: &System, root: Pid) -> Option<u64> {
    let _root_is_alive = system.process(root)?;
    Some(
        system
            .processes()
            .values()
            .filter(|process| descends_from(system, process.pid(), root))
            .map(sysinfo::Process::accumulated_cpu_time)
            .sum(),
    )
}

/// Whether `pid` is `root` or has it somewhere up its parent chain.
fn descends_from(system: &System, pid: Pid, root: Pid) -> bool {
    let mut current = Some(pid);
    for _ in 0..MAX_ANCESTRY_DEPTH {
        match current {
            Some(found) if found == root => return true,
            Some(found) => current = system.process(found).and_then(sysinfo::Process::parent),
            None => return false,
        }
    }
    false
}
