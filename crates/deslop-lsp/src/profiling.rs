//! Feature-gated CPU profiling support for `deslop-lsp`.
//!
//! [LIVE-PROFILING] The default binary has no profiler dependency. Builds
//! compiled with the `profiling` feature can set `DESLOP_PROFILE_DIR` and
//! get an attachable Firefox-profiler JSON profile when the LSP exits
//! cleanly.

#[cfg(all(feature = "profiling", unix))]
use std::{
    collections::{hash_map::Entry, HashMap},
    env, fmt,
    fs::{self, File},
    io::BufWriter,
    path::{Path, PathBuf},
    process,
    time::SystemTime,
};

#[cfg(all(feature = "profiling", unix))]
use anyhow::Result;

#[cfg(all(feature = "profiling", unix))]
use fxprof_processed_profile::{
    CategoryHandle, CpuDelta, Frame, FrameFlags, FrameInfo, ProcessHandle, Profile,
    SamplingInterval, ThreadHandle, Timestamp,
};

/// Environment variable that enables built-in LSP profiling.
#[cfg(all(feature = "profiling", unix))]
const PROFILE_DIR_ENV: &str = "DESLOP_PROFILE_DIR";

/// Output sample rate used by the signal profiler.
#[cfg(all(feature = "profiling", unix))]
const SAMPLE_FREQUENCY_HZ: i32 = 99;

/// Firefox profile sampling interval corresponding to the pprof rate.
#[cfg(all(feature = "profiling", unix))]
const SAMPLE_INTERVAL_MS: u64 = 10;
/// Process name recorded in the emitted Firefox profile.
#[cfg(all(feature = "profiling", unix))]
const PROCESS_NAME: &str = "deslop-lsp";
/// Profile start time, in milliseconds, that samples are offset from.
#[cfg(all(feature = "profiling", unix))]
const REFERENCE_START_MS: f64 = 0.0;

/// RAII holder for the optional process-wide CPU profiler.
#[cfg(all(feature = "profiling", unix))]
pub(crate) struct LspProfileGuard {
    /// Active profile state when profiling was requested and successfully started.
    active: Option<ActiveProfile>,
}

#[cfg(all(feature = "profiling", unix))]
impl fmt::Debug for LspProfileGuard {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LspProfileGuard")
            .field("active", &self.active)
            .finish()
    }
}

#[cfg(all(feature = "profiling", unix))]
impl LspProfileGuard {
    /// Starts profiling from `DESLOP_PROFILE_DIR`, returning a no-op guard when unset.
    pub(crate) fn from_env() -> Self {
        let Some(profile_dir) = env::var_os(PROFILE_DIR_ENV) else {
            return Self { active: None };
        };
        let profile_dir = PathBuf::from(profile_dir);
        match ActiveProfile::start(&profile_dir) {
            Ok(active) => Self {
                active: Some(active),
            },
            Err(error) => {
                tracing::warn!(%error, "deslop-lsp CPU profiler did not start");
                Self { active: None }
            }
        }
    }
}

/// Started `pprof-rs` guard plus its Firefox-profile output path.
#[cfg(all(feature = "profiling", unix))]
struct ActiveProfile {
    /// Process-wide signal sampler.
    guard: pprof::ProfilerGuard<'static>,
    /// Final JSON path written on clean shutdown.
    output_path: PathBuf,
    /// Profile reference time used in Firefox profiler metadata.
    started_at: SystemTime,
}

#[cfg(all(feature = "profiling", unix))]
impl fmt::Debug for ActiveProfile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ActiveProfile")
            .field("output_path", &self.output_path)
            .field("started_at", &self.started_at)
            .finish_non_exhaustive()
    }
}

#[cfg(all(feature = "profiling", unix))]
impl ActiveProfile {
    /// Creates the output directory and starts the signal profiler.
    fn start(profile_dir: &Path) -> Result<Self> {
        fs::create_dir_all(profile_dir)?;
        let output_path =
            profile_dir.join(format!("deslop-lsp-{}-firefox-profile.json", process::id()));
        let guard = pprof::ProfilerGuardBuilder::default()
            .frequency(SAMPLE_FREQUENCY_HZ)
            .build()?;
        tracing::info!(
            profile = %output_path.display(),
            "deslop-lsp CPU profiler started",
        );
        Ok(Self {
            guard,
            output_path,
            started_at: SystemTime::now(),
        })
    }
}

#[cfg(all(feature = "profiling", unix))]
impl Drop for ActiveProfile {
    fn drop(&mut self) {
        match self.guard.report().build() {
            Ok(report) => {
                match write_firefox_profile(&report, self.started_at, &self.output_path) {
                    Ok(()) => {
                        tracing::info!(
                            profile = %self.output_path.display(),
                            "deslop-lsp CPU profile written",
                        );
                    }
                    Err(error) => {
                        tracing::warn!(%error, "deslop-lsp CPU profile write failed");
                    }
                }
            }
            Err(error) => {
                tracing::warn!(%error, "deslop-lsp CPU profile report failed");
            }
        }
    }
}

/// Serializes a pprof stack-count report as Firefox processed-profile JSON.
#[cfg(all(feature = "profiling", unix))]
fn write_firefox_profile(
    report: &pprof::Report,
    started_at: SystemTime,
    output_path: &Path,
) -> Result<()> {
    let profile = profile_from_report(report, started_at);
    let output = File::create(output_path)?;
    let writer = BufWriter::new(output);
    serde_json::to_writer(writer, &profile)?;
    Ok(())
}

/// Converts pprof stack counts into the Firefox processed-profile model.
#[cfg(all(feature = "profiling", unix))]
fn profile_from_report(report: &pprof::Report, started_at: SystemTime) -> Profile {
    let mut profile = Profile::new(
        PROCESS_NAME,
        started_at.into(),
        SamplingInterval::from_millis(SAMPLE_INTERVAL_MS),
    );
    let process_handle = profile.add_process(
        PROCESS_NAME,
        process::id(),
        Timestamp::from_millis_since_reference(REFERENCE_START_MS),
    );
    let mut threads = HashMap::new();
    let mut has_sample = false;
    for (frames, count) in &report.data {
        if *count <= 0 {
            continue;
        }
        let weight = i32::try_from(*count).unwrap_or(i32::MAX);
        let thread = thread_for_frames(&mut profile, process_handle, &mut threads, frames);
        let stack = stack_for_frames(&mut profile, thread, frames);
        profile.add_sample(
            thread,
            Timestamp::from_millis_since_reference(REFERENCE_START_MS),
            stack,
            CpuDelta::ZERO,
            weight,
        );
        has_sample = true;
    }
    if !has_sample {
        add_no_sample_placeholder(&mut profile, process_handle);
    }
    profile
}

/// Returns the Firefox thread handle for a sampled pprof thread.
#[cfg(all(feature = "profiling", unix))]
fn thread_for_frames(
    profile: &mut Profile,
    process_handle: ProcessHandle,
    threads: &mut HashMap<u32, ThreadHandle>,
    frames: &pprof::Frames,
) -> ThreadHandle {
    let tid = firefox_thread_id(frames.thread_id);
    match threads.entry(tid) {
        Entry::Occupied(entry) => *entry.get(),
        Entry::Vacant(entry) => {
            let thread = profile.add_thread(
                process_handle,
                tid,
                Timestamp::from_millis_since_reference(REFERENCE_START_MS),
                false,
            );
            profile.set_thread_name(thread, &frames.thread_name_or_id());
            *entry.insert(thread)
        }
    }
}

/// Converts a pprof thread identifier into Firefox profiler's `u32` tid shape.
#[cfg(all(feature = "profiling", unix))]
fn firefox_thread_id(thread_id: u64) -> u32 {
    u32::try_from(thread_id).unwrap_or_else(|_| process::id())
}

/// Interns a pprof stack in the Firefox profile.
#[cfg(all(feature = "profiling", unix))]
fn stack_for_frames(
    profile: &mut Profile,
    thread: ThreadHandle,
    frames: &pprof::Frames,
) -> Option<fxprof_processed_profile::StackHandle> {
    let mut stack_frames = Vec::new();
    for frame in frames.frames.iter().rev() {
        for symbol in frame.iter().rev() {
            let symbol_name = symbol.name();
            let label = profile.intern_string(&symbol_name);
            stack_frames.push(FrameInfo {
                frame: Frame::Label(label),
                category_pair: CategoryHandle::OTHER.into(),
                flags: FrameFlags::empty(),
            });
        }
    }
    profile.intern_stack_frames(thread, stack_frames.into_iter())
}

/// Adds a placeholder sample so an idle repro still produces an attachable file.
#[cfg(all(feature = "profiling", unix))]
fn add_no_sample_placeholder(profile: &mut Profile, process_handle: ProcessHandle) {
    let thread = profile.add_thread(
        process_handle,
        process::id(),
        Timestamp::from_millis_since_reference(REFERENCE_START_MS),
        true,
    );
    profile.set_thread_name(thread, PROCESS_NAME);
    let label = profile.intern_string("no pprof samples captured");
    let stack = profile.intern_stack_frames(
        thread,
        [FrameInfo {
            frame: Frame::Label(label),
            category_pair: CategoryHandle::OTHER.into(),
            flags: FrameFlags::empty(),
        }]
        .into_iter(),
    );
    profile.add_sample(
        thread,
        Timestamp::from_millis_since_reference(REFERENCE_START_MS),
        stack,
        CpuDelta::ZERO,
        1,
    );
}

#[cfg(all(feature = "profiling", unix, test))]
mod tests;

/// No-op profiler holder for default builds and non-Unix targets.
#[cfg(not(all(feature = "profiling", unix)))]
#[derive(Debug)]
pub(crate) struct LspProfileGuard;

#[cfg(not(all(feature = "profiling", unix)))]
impl LspProfileGuard {
    /// Returns a no-op guard when profiling support is not compiled in.
    pub(crate) fn from_env() -> Self {
        Self
    }
}
