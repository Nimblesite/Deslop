//! [LIVE-PROFILING] Unit tests for the pprof → Firefox-profile conversion.
//!
//! The conversion is the part of profiling a person actually depends on: the
//! sampler is `pprof`'s, but whether the file it produces opens in the
//! Firefox profiler is this module's answer. An E2E cannot ask that question
//! cheaply — it would have to run a real signal profiler and hope the machine
//! scheduled a sample — so the report is built here by hand and the profile
//! is read back.
//!
//! The profile is read back as a parsed document, never as text: the thread
//! list, its string table and its sample count are the claims being made, and
//! a substring search over serialised JSON would also match a name that
//! happened to appear in unrelated metadata.
//!
//! Every stack below is authored, so the same input always produces the same
//! profile ([PIPELINE-DETERMINISM]): the reference time is the epoch, never
//! `SystemTime::now()`.

use std::{collections::HashMap, time::Duration};

use anyhow::{anyhow, Result};
use pprof::{Frames, Report, Symbol};
use serde_json::Value;

use super::{
    firefox_thread_id, profile_from_report, write_firefox_profile, LspProfileGuard, PROCESS_NAME,
};

/// The label the placeholder sample carries. A profile with no samples at all
/// is rejected by the Firefox profiler as malformed, so an idle repro has to
/// carry one frame that says why it is empty.
const PLACEHOLDER_LABEL: &str = "no pprof samples captured";

/// How an idle guard must render, so an unprofiled run is distinguishable in
/// a log from one whose profiler failed to start.
const IDLE_GUARD_DEBUG: &str = "LspProfileGuard { active: None }";

/// Thread identity shared by every authored stack, so a profile that created
/// one thread per stack is distinguishable from one that reused a thread.
const THREAD_NAME: &str = "deslop-analyser";
const THREAD_ID: u64 = 7;

/// Two distinct symbol names, so each authored stack is visible in the output
/// independently of the order the report's map iterates in.
const FIRST_SYMBOL: &str = "deslop_core::pipeline::fingerprint";
const SECOND_SYMBOL: &str = "deslop_core::pipeline::cluster";

/// A sample seen once, and one the profiler recorded but never observed.
const OBSERVED_ONCE: isize = 1;
const NEVER_OBSERVED: isize = 0;

/// Sample and thread counts the assertions below expect.
const ONE_THREAD: usize = 1;
const ONE_SAMPLE: u64 = 1;
const TWO_SAMPLES: u64 = 2;

/// A thread id wider than the Firefox profiler's `u32` thread field.
const TOO_WIDE_THREAD_ID: u64 = u64::MAX;
/// A thread id that fits, and therefore must survive unchanged.
const NARROW_THREAD_ID: u32 = 4_242;

/// The name of the written profile inside its temporary directory.
const PROFILE_FILE: &str = "firefox-profile.json";

/// Field names in the Firefox processed-profile document.
const THREADS: &str = "threads";
const THREAD_NAME_FIELD: &str = "name";
const THREAD_ID_FIELD: &str = "tid";
const STRING_TABLE: &str = "stringArray";
const SAMPLES: &str = "samples";
const LENGTH: &str = "length";

/// The reference instant every profile below is anchored to.
fn reference_time() -> std::time::SystemTime {
    std::time::SystemTime::UNIX_EPOCH
}

/// `pprof::ReportTiming` lives in a private module and cannot be named here,
/// so inference builds it. Nothing below reads it.
fn defaulted<T: Default>() -> T {
    T::default()
}

/// One authored stack on [`THREAD_ID`], distinguished by `symbol` and by
/// `offset` so two stacks are never the same map key.
fn stack(symbol: &str, offset: u64) -> Frames {
    Frames {
        frames: vec![vec![Symbol {
            name: Some(symbol.as_bytes().to_vec()),
            addr: None,
            lineno: None,
            filename: None,
        }]],
        thread_name: THREAD_NAME.to_owned(),
        thread_id: THREAD_ID,
        sample_timestamp: reference_time()
            .checked_add(Duration::from_secs(offset))
            .unwrap_or_else(reference_time),
    }
}

/// A report holding exactly the `(stack, count)` pairs given.
fn report(samples: Vec<(Frames, isize)>) -> Report {
    Report {
        data: samples.into_iter().collect::<HashMap<Frames, isize>>(),
        timing: defaulted(),
    }
}

/// The profile as a parsed document, which is how the profiler reads it.
fn profile_of(report: &Report) -> Result<Value> {
    Ok(serde_json::to_value(profile_from_report(
        report,
        reference_time(),
    ))?)
}

/// The profile's only thread, or why there is not exactly one.
fn one_thread(profile: &Value) -> Result<&Value> {
    let threads = profile
        .get(THREADS)
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("the profile has no thread list: {profile}"))?;
    if threads.len() != ONE_THREAD {
        return Err(anyhow!(
            "expected one thread, found {}: {profile}",
            threads.len()
        ));
    }
    threads
        .first()
        .ok_or_else(|| anyhow!("the thread list is empty: {profile}"))
}

/// A thread's string table — every symbol and label the thread refers to.
fn symbols_of(thread: &Value) -> Result<Vec<&str>> {
    thread
        .get(STRING_TABLE)
        .and_then(Value::as_array)
        .map(|table| table.iter().filter_map(Value::as_str).collect())
        .ok_or_else(|| anyhow!("the thread has no string table: {thread}"))
}

/// A thread's string table in a fixed order, so the report map's iteration
/// order cannot decide whether an assertion passes.
fn sorted_symbols_of(thread: &Value) -> Result<Vec<&str>> {
    let mut symbols = symbols_of(thread)?;
    symbols.sort_unstable();
    Ok(symbols)
}

/// How many samples a thread recorded.
fn samples_of(thread: &Value) -> Result<u64> {
    thread
        .get(SAMPLES)
        .and_then(|samples| samples.get(LENGTH))
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("the thread records no sample count: {thread}"))
}

/// Asserts a thread carries the identity every authored stack was sampled on.
fn assert_authored_thread(thread: &Value, profile: &Value) -> Result<()> {
    assert_eq!(
        text_of(thread, THREAD_ID_FIELD)?,
        THREAD_ID.to_string(),
        "both stacks were sampled on thread {THREAD_ID}, and the surviving \
         thread must be that one: {profile}"
    );
    assert_eq!(text_of(thread, THREAD_NAME_FIELD)?, THREAD_NAME, "{profile}");
    Ok(())
}

/// A named string field on a thread.
fn text_of<'a>(thread: &'a Value, field: &str) -> Result<&'a str> {
    thread
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("the thread has no {field}: {thread}"))
}

#[test]
fn an_idle_run_still_produces_a_profile_that_opens() -> Result<()> {
    let profile = profile_of(&report(Vec::new()))?;
    let thread = one_thread(&profile)?;
    assert_eq!(
        symbols_of(thread)?,
        vec![PLACEHOLDER_LABEL],
        "a profile with no samples is rejected as malformed, so the one frame \
         it carries must be the placeholder that says why it is empty: {profile}"
    );
    assert_eq!(
        text_of(thread, THREAD_NAME_FIELD)?,
        PROCESS_NAME,
        "the placeholder thread must still be named after the process it \
         profiled, or the file opens with an anonymous thread: {profile}"
    );
    assert_eq!(samples_of(thread)?, ONE_SAMPLE, "{profile}");
    Ok(())
}

#[test]
fn a_stack_the_profiler_never_observed_is_not_a_sample() -> Result<()> {
    let profile = profile_of(&report(vec![(stack(FIRST_SYMBOL, 0), NEVER_OBSERVED)]))?;
    let thread = one_thread(&profile)?;
    assert_eq!(
        symbols_of(thread)?,
        vec![PLACEHOLDER_LABEL],
        "a stack with a zero count was never on CPU; recording it would put \
         weight on code that never ran, and dropping it leaves only the \
         placeholder: {profile}"
    );
    assert_eq!(samples_of(thread)?, ONE_SAMPLE, "{profile}");
    Ok(())
}

#[test]
fn two_stacks_from_one_thread_share_one_profile_thread() -> Result<()> {
    let profile = profile_of(&report(vec![
        (stack(FIRST_SYMBOL, 0), OBSERVED_ONCE),
        (stack(SECOND_SYMBOL, 1), OBSERVED_ONCE),
    ]))?;
    let thread = one_thread(&profile)?;
    assert_authored_thread(thread, &profile)?;
    assert_eq!(
        sorted_symbols_of(thread)?,
        vec![SECOND_SYMBOL, FIRST_SYMBOL],
        "both sampled frames must survive the merge onto one thread: {profile}"
    );
    assert_eq!(samples_of(thread)?, TWO_SAMPLES, "{profile}");
    Ok(())
}

#[test]
fn a_thread_id_too_wide_for_the_profile_falls_back_to_the_process() {
    assert_eq!(
        firefox_thread_id(u64::from(NARROW_THREAD_ID)),
        NARROW_THREAD_ID,
        "a thread id that fits must be carried through unchanged, or samples \
         are attributed to a thread nobody can find"
    );
    assert_eq!(
        firefox_thread_id(TOO_WIDE_THREAD_ID),
        std::process::id(),
        "the Firefox profile models a thread id as a u32; a wider id must \
         land on the process rather than wrap onto an unrelated thread"
    );
}

#[test]
fn the_written_profile_is_json_on_disk() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let output_path = directory.path().join(PROFILE_FILE);
    write_firefox_profile(
        &report(vec![(stack(FIRST_SYMBOL, 0), OBSERVED_ONCE)]),
        reference_time(),
        &output_path,
    )?;

    let profile: Value = serde_json::from_str(&std::fs::read_to_string(&output_path)?)?;
    let thread = one_thread(&profile)?;
    assert_eq!(
        symbols_of(thread)?,
        vec![FIRST_SYMBOL],
        "the sampled frame must survive the round trip to disk: {profile}"
    );
    assert_eq!(samples_of(thread)?, ONE_SAMPLE, "{profile}");
    Ok(())
}

#[test]
fn a_run_that_never_started_the_profiler_says_so() {
    let idle = LspProfileGuard { active: None };
    assert_eq!(
        format!("{idle:?}"),
        IDLE_GUARD_DEBUG,
        "a guard with no profiler must render as one; a Debug that hid the \
         difference would make an unprofiled run indistinguishable in a log"
    );
}
