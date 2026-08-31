//! TypeScript-specific E2E tests ([LANG-CAND-TYPESCRIPT],
//! [PIPELINE-NORMALIZE-AST]).
//!
//! These prove TypeScript-only syntax parses through
//! `tree-sitter-typescript` and normalises so that user-defined type
//! identifiers are rename-invariant while the type *structure* survives.
//! Generics, interfaces, enums, decorated classes, primitive and named
//! type annotations, and qualified (dotted) type names are each exercised
//! by a renamed clone, and the bucket asserted is the real engine output.

use anyhow::Result;

use crate::common::*;

#[test]
fn typescript_generic_functions_with_renamed_type_params_cluster() -> Result<()> {
    // Two generic container helpers, every type parameter and value
    // identifier renamed; the generic shape is preserved so the token layer
    // stays invariant and the clone is `nearly_identical`.
    assert_bucketed_clone("ts-generics", 12, &["cache.ts", "store.ts"], false)
}

#[test]
fn typescript_unrelated_interfaces_are_suppressed_as_data_shape() -> Result<()> {
    // Two unrelated interfaces with the same field *shape* (readonly,
    // optional, `ReadonlyArray`, a nested object, a union, `Date | null`) but
    // different field *names* are distinct domain types, not extractable
    // duplicate logic. Like renamed-field Rust structs (#224), the data-shape
    // filter suppresses them so they never pollute top-offenders — even
    // though both files are analysed.
    let report = run_report(&fixture("ts-interfaces"), 10)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(2),
        "both interface files must be analysed: {report:#}"
    );
    assert!(
        clusters(&report).is_empty(),
        "two unrelated interface data shapes must not be reported as a clone: {report:#}"
    );
    assert!(
        clusters_hidden(&report) >= 1,
        "the data-shape family must be detected and hidden, proving the filter fired: {report:#}"
    );
    Ok(())
}

#[test]
fn typescript_decorated_classes_clone_is_nearly_identical() -> Result<()> {
    assert_bucketed_clone(
        "ts-decorators",
        12,
        &["order.controller.ts", "user.controller.ts"],
        false,
    )
}

#[test]
fn typescript_enums_with_renamed_members_cluster() -> Result<()> {
    assert_bucketed_clone("ts-enums", 10, &["http-status.ts", "task-state.ts"], false)
}

#[test]
fn typescript_primitive_type_annotation_difference_still_clusters() -> Result<()> {
    // The two functions differ only in their primitive type annotations
    // (`string`/`number` vs `any`/`boolean`); `predefined_type` keywords
    // normalise to one structural kind, so the bodies still match. Every
    // name and literal agrees, so the content gate ([FUSED-CONTENT-GATE])
    // confirms the pair as a genuine near-miss rather than the shape-only
    // routing the token-layer fallback used to force.
    assert_bucketed_clone(
        "ts-type-keyword",
        10,
        &["render-a.ts", "render-b.ts"],
        false,
    )
}

#[test]
fn typescript_named_type_alias_rename_is_token_invariant() -> Result<()> {
    // `type Widget` vs `type Gadget`: a user-defined `type_identifier`
    // collapses to a placeholder, so renaming the alias leaves the clone
    // fully token-supported.
    assert_bucketed_clone(
        "ts-named-type-rename",
        10,
        &["build-a.ts", "build-b.ts"],
        false,
    )
}

#[test]
fn typescript_qualified_type_name_rename_is_token_invariant() -> Result<()> {
    // `Intl.DateTimeFormat` vs `Temporal.Instant`: a qualified type name is
    // a structural `nested_type_identifier` whose leaf segments collapse, so
    // renaming both segments keeps the clone detectable — the behaviour the
    // leaf-only identifier-collapse rule guarantees.
    assert_bucketed_clone(
        "ts-qualified-type-rename",
        8,
        &["alpha.ts", "beta.ts"],
        false,
    )
}
