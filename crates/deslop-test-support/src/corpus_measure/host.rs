//! [CORPUS-MEASURE-HOST] A measured scan shares its machine with editors,
//! builds and other agents, and its own timing is worthless when another
//! measured scan runs beside it. So every measured scan runs at the lowest
//! priority the platform offers, one at a time across the whole machine,
//! inside a deadline, and records the load it started under.

use std::{
    fs::{self, OpenOptions},
    io::{ErrorKind, Write as _},
    path::PathBuf,
    sync::mpsc::{self, RecvTimeoutError, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use super::cpu::descends_from;

#[cfg(all(test, unix))]
#[path = "host/tests.rs"]
mod tests;

/// The POSIX niceness every measured scan runs at: the lowest priority.
pub const SCAN_NICENESS: &str = "19";
/// The program that lowers it, by absolute path so `PATH` cannot swap it.
pub const NICE_PROGRAM: &str = "/usr/bin/nice";
/// Windows' `BELOW_NORMAL_PRIORITY_CLASS` process creation flag.
#[cfg(windows)]
pub const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
/// The longest one measured scan may run before its process tree is killed.
/// Every corpus ceiling sits far below it, so only a hung scan reaches it.
pub const SCAN_DEADLINE: Duration = Duration::from_secs(3_600);
/// How often a run waiting on the machine-wide lock looks again.
const LOCK_POLL: Duration = Duration::from_secs(1);
/// The lock every measured scan on this machine takes, whatever checkout
/// it runs from. Fixed rather than per-user `TMPDIR`, which differs
/// between sessions.
const POSIX_LOCK_PATH: &str = "/tmp/deslop-corpus-measure.lock";
/// The same lock's name under the Windows temp directory.
const WINDOWS_LOCK_NAME: &str = "deslop-corpus-measure.lock";

/// Where the machine-wide measurement lock lives.
fn lock_path() -> PathBuf {
    if cfg!(windows) {
        std::env::temp_dir().join(WINDOWS_LOCK_NAME)
    } else {
        PathBuf::from(POSIX_LOCK_PATH)
    }
}

/// The machine-wide measurement lock, released when dropped.
#[derive(Debug)]
pub struct MeasureLock {
    /// The lock file this process created.
    path: PathBuf,
}

impl MeasureLock {
    /// Waits until no other live process holds the lock, then takes it. A
    /// lock left by a process that has exited is taken over.
    ///
    /// # Errors
    ///
    /// Returns an error when the lock file cannot be created or read.
    pub fn acquire() -> Result<Self> {
        Self::acquire_at(lock_path())
    }

    /// [`Self::acquire`] at `path`, the seam that lets a test hold a lock
    /// no real measurement shares.
    fn acquire_at(path: PathBuf) -> Result<Self> {
        let mut announced = false;
        loop {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    write!(file, "{}", std::process::id())
                        .with_context(|| format!("writing {}", path.display()))?;
                    return Ok(Self { path });
                }
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                    let holder = fs::read_to_string(&path).unwrap_or_default();
                    if !is_live(holder.trim()) {
                        let _stale = fs::remove_file(&path);
                        continue;
                    }
                    if !announced {
                        eprintln!(
                            "==> waiting for the measured scan of pid {} to finish ({})",
                            holder.trim(),
                            path.display()
                        );
                        announced = true;
                    }
                    thread::sleep(LOCK_POLL);
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("creating {}", path.display()))
                }
            }
        }
    }
}

impl Drop for MeasureLock {
    fn drop(&mut self) {
        let _released = fs::remove_file(&self.path);
    }
}

/// Whether `pid` names a running process. Anything unreadable is not.
fn is_live(pid: &str) -> bool {
    let Ok(pid) = pid.parse::<u32>() else {
        return false;
    };
    let pid = Pid::from_u32(pid);
    let mut system = System::new();
    let _updated = system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing(),
    );
    system.process(pid).is_some()
}

/// The machine's load when a measured scan started.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HostLoad {
    /// The one-minute load average. Windows reports none, so it reads 0.
    pub one_minute: f64,
    /// Logical cores, the scale the load average is read against.
    pub logical_cores: usize,
}

impl HostLoad {
    /// The load right now.
    #[must_use]
    pub fn now() -> Self {
        Self {
            one_minute: System::load_average().one,
            logical_cores: std::thread::available_parallelism().map_or(1, usize::from),
        }
    }

    /// More runnable work than cores: every timing taken now is contended.
    #[must_use]
    pub fn is_contended(self) -> bool {
        self.one_minute > f64::from(u32::try_from(self.logical_cores).unwrap_or(u32::MAX))
    }
}

/// Kills a measured scan's whole process tree once its deadline passes.
#[derive(Debug)]
pub struct Deadline {
    /// Dropping or sending on this disarms the watchdog.
    stop: Sender<()>,
    /// The watchdog, which reports whether it had to kill the tree.
    worker: JoinHandle<bool>,
}

impl Deadline {
    /// Starts watching the tree headed by `root`, killing it after `after`.
    #[must_use]
    pub fn arm(root: u32, after: Duration) -> Self {
        let (stop, stopped) = mpsc::channel();
        let worker = thread::spawn(move || {
            let expired = matches!(stopped.recv_timeout(after), Err(RecvTimeoutError::Timeout));
            if expired {
                kill_tree(Pid::from_u32(root));
            }
            expired
        });
        Self { stop, worker }
    }

    /// Disarms the watchdog once the scan has exited.
    ///
    /// # Errors
    ///
    /// Returns an error naming the deadline when the watchdog killed the
    /// scan, or when the watchdog thread panicked.
    pub fn disarm(self, after: Duration) -> Result<()> {
        let _stopped = self.stop.send(());
        let killed = self
            .worker
            .join()
            .map_err(|_| anyhow!("the scan deadline watchdog panicked"))?;
        if killed {
            return Err(anyhow!(
                "the measured scan ran past its {} s deadline and its process tree was killed",
                after.as_secs()
            ));
        }
        Ok(())
    }
}

/// Kills `root` and every process descended from it.
fn kill_tree(root: Pid) {
    let mut system = System::new();
    let _updated = system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing(),
    );
    let tree: Vec<Pid> = system
        .processes()
        .keys()
        .copied()
        .filter(|pid| descends_from(&system, *pid, root))
        .collect();
    for pid in tree {
        if let Some(process) = system.process(pid) {
            let _signalled = process.kill();
        }
    }
}
