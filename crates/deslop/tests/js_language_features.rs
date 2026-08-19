//! JavaScript language-feature E2E tests ([LANG-CAND-JAVASCRIPT],
//! [PIPELINE-NORMALIZE-AST]).
//!
//! Each fixture is a renamed clone built around one JavaScript feature —
//! classes, async/await, generators, template and tagged-template literals,
//! optional chaining, destructuring, and regex literals — proving the
//! feature parses through `tree-sitter-javascript` and that identifier and
//! literal normalisation keeps the clone detectable across the rename. The
//! bucket asserted for each follows [FUSION-CONTENT-GATE]: measured
//! content evidence decides. A rename whose surviving content corroborates
//! its identifier mapping reaches the act-now `nearly_identical` bucket.
//! `js-classes` is the maximal case (#409): a total, repeated bijection
//! (`balance -> funds`, `amount -> value`, `deposit -> credit`) whose
//! literals *echo* the same substitutions byte for byte — the rename done
//! thoroughly is proof of copying, not disagreement, so the pair renders
//! `nearly_identical` even though almost no position is byte-equal.

use anyhow::Result;

mod common;
use crate::common::*;

#[test]
fn javascript_class_method_clone_is_a_proven_rename() -> Result<()> {
    // Account/Wallet is one algorithm under two vocabularies: every
    // identifier substitutes consistently and with repetition, and three
    // of the four string literals transform by exactly those
    // substitutions ("amount must be positive" -> "value must be
    // positive" echoes `amount -> value`). Before #409 the blind literal
    // count read those echoes as disproof and demoted the pair to
    // `structural_only` — a false negative on a textbook Type-2 clone.
    assert_bucketed_clone(
        "js-classes",
        8,
        &["account.js", "wallet.js"],
        "nearly_identical",
    )
}

#[test]
fn javascript_async_await_clone_is_detected() -> Result<()> {
    assert_bucketed_clone(
        "js-async",
        8,
        &["fetch_team.js", "fetch_user.js"],
        "structural_only",
    )
}

#[test]
fn javascript_generator_clone_is_nearly_identical() -> Result<()> {
    assert_bucketed_clone(
        "js-generators",
        8,
        &["range_gen.js", "walk_gen.js"],
        "nearly_identical",
    )
}

#[test]
fn javascript_template_literal_clone_is_detected() -> Result<()> {
    // Anchor-rich by the module contract above: every template chunk
    // ("Hello ", " totalling ", " dollars has shipped.", …) survives the
    // rename verbatim, and `firstName`/`lastName`/`id`/`total`/
    // `trackingUrl`/`email` map through one substitution. That is a proven
    // Type-2 rename, so [FUSION-CONTENT-GATE] must keep it out of the
    // demoted bucket — the exact promotion the content gate exists to make.
    assert_bucketed_clone(
        "js-template-literals",
        8,
        &["render_email.js", "render_receipt.js"],
        "nearly_identical",
    )
}

#[test]
fn javascript_tagged_template_clone_is_nearly_identical() -> Result<()> {
    assert_bucketed_clone(
        "js-tagged-templates",
        8,
        &["group_query.js", "user_query.js"],
        "nearly_identical",
    )
}

#[test]
fn javascript_optional_chaining_clone_is_detected() -> Result<()> {
    // Anchor-rich: `3000`, `5` and "default" are preserved, and so is every
    // accessed property name (`network`, `timeout`, `retries`, `max`,
    // `meta`, `name`, `trim`) — only the bound locals are renamed. Pooled
    // content agreement therefore vouches for the pair; no literal-count
    // threshold is involved ([REPAIR-RENAME-ANCHOR-MASS] deleted the
    // anchor floor).
    assert_bucketed_clone(
        "js-optional-chaining",
        8,
        &["read_config.js", "read_options.js"],
        "nearly_identical",
    )
}

#[test]
fn javascript_destructuring_clone_is_detected() -> Result<()> {
    assert_bucketed_clone(
        "js-destructuring",
        8,
        &["build_point.js", "build_vertex.js"],
        "nearly_identical",
    )
}

#[test]
fn javascript_regex_literal_clone_is_detected() -> Result<()> {
    assert_bucketed_clone(
        "js-regex",
        8,
        &["validate_email.js", "validate_handle.js"],
        "nearly_identical",
    )
}
