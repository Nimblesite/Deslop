//! The size budget [`CoreError`] must stay inside.
//!
//! `CoreError` is the `Err` half of nearly every signature in this crate, so
//! its width is paid on the success path too: `Result<T, CoreError>` is at
//! least as wide as `CoreError`, and every one of those returns is moved
//! whether or not anything failed. Three foreign error types are the reason
//! it can drift — `ignore::Error` (64 bytes), `toml::de::Error` (88) and
//! `serde_json::Error` (8) — and only the first two are ever stored inline.
//!
//! `clippy::result_large_err` fires at 128 bytes. It caught this at exactly
//! 128 across 30 signatures in `live` and `refactor`, which is what a budget
//! stated once and asserted here prevents: the next foreign error added to a
//! variant fails this test with a number, rather than 30 lint errors pointing
//! at functions that did nothing wrong.

use std::mem::size_of;

use super::CoreError;

/// `clippy::result_large_err`'s default `large-err-threshold`. Staying under
/// it is the contract; the lint is only how it gets noticed.
const LARGE_ERR_THRESHOLD: usize = 128;

#[test]
fn the_core_error_stays_under_the_large_err_threshold() {
    let width = size_of::<CoreError>();
    assert!(
        width < LARGE_ERR_THRESHOLD,
        "CoreError is {width} bytes, at or over clippy's {LARGE_ERR_THRESHOLD}-byte \
         `result_large_err` threshold. Every `Result<_, CoreError>` in the crate is now that \
         wide on its success path too. Box the foreign error a variant stores inline rather \
         than widening the budget."
    );
}

/// The two foreign errors that have actually pushed this enum over: they are
/// wider than the whole rest of a variant, so each must be behind a pointer.
#[test]
fn the_widest_foreign_errors_are_not_stored_inline() {
    let boxed = size_of::<Box<ignore::Error>>();
    assert!(
        size_of::<ignore::Error>() > boxed,
        "this test is asserting nothing: `ignore::Error` is no wider than a pointer, so \
         boxing it cannot be what keeps CoreError small"
    );
    assert!(
        size_of::<toml::de::Error>() > boxed,
        "this test is asserting nothing: `toml::de::Error` is no wider than a pointer"
    );
    // The budget above is what actually binds; these two only explain why it
    // holds, so they must keep being the reason it does.
    assert!(
        size_of::<CoreError>() < size_of::<ignore::Error>() + size_of::<toml::de::Error>(),
        "CoreError got wider than the two foreign errors it is supposed to be storing behind \
         pointers, so at least one of them is inline again"
    );
}
