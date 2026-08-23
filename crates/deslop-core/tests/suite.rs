//! [CI-RELEASE-BUILD] [TEST-ONE-BINARY] The crate's whole integration
//! suite, linked once.
//!
//! Cargo builds one executable per `tests/*.rs`, and each one statically
//! links the entire workspace under the release profile. At 176 files that
//! was 176 whole-program links per CI run — the bulk of a 20-minute compile
//! that cancelled every Rust shard at its cap. Declaring the suites as
//! modules of a single binary links them once instead.
//!
//! Each module below is a former `tests/*.rs`, unchanged apart from its
//! `mod common;` line: the shared helpers are declared here, once, so
//! `crate::common::…` still resolves from every suite.

/// Shared fixture helpers, declared once for every suite below.
mod common;

#[path = "cluster_overlap_collapse.rs"]
mod cluster_overlap_collapse;
#[path = "cluster_subsumption.rs"]
mod cluster_subsumption;
#[path = "cross_language_threshold.rs"]
mod cross_language_threshold;
#[path = "diff_render_tags.rs"]
mod diff_render_tags;
#[path = "embedding_pairs.rs"]
mod embedding_pairs;
#[path = "embedding_pass_observability.rs"]
mod embedding_pass_observability;
#[path = "issue_117.rs"]
mod issue_117;
#[path = "issue_121_pytest_fixture_boilerplate.rs"]
mod issue_121_pytest_fixture_boilerplate;
#[path = "issue_124_node_count.rs"]
mod issue_124_node_count;
#[path = "issue_239_csharp_reparse.rs"]
mod issue_239_csharp_reparse;
#[path = "issue_270_seed_language_drift.rs"]
mod issue_270_seed_language_drift;
#[path = "issue_287_live_ingest_gitignore_parity.rs"]
mod issue_287_live_ingest_gitignore_parity;
#[path = "issue_299_no_op_pass_skips_rerender.rs"]
mod issue_299_no_op_pass_skips_rerender;
#[path = "issue_336_deep_ast_stack_overflow.rs"]
mod issue_336_deep_ast_stack_overflow;
#[path = "issue_45_observability.rs"]
mod issue_45_observability;
#[path = "issue_82_embedding_context_budget.rs"]
mod issue_82_embedding_context_budget;
#[path = "issue_91_embedding_roi.rs"]
mod issue_91_embedding_roi;
#[path = "issue_93_embedding_uniqueness.rs"]
mod issue_93_embedding_uniqueness;
#[path = "issue_98_99_108_120_122_thresholds.rs"]
mod issue_98_99_108_120_122_thresholds;
#[path = "lang_registry_vsix_parity.rs"]
mod lang_registry_vsix_parity;
#[path = "live.rs"]
mod live;
#[path = "live_delta_field_coverage.rs"]
mod live_delta_field_coverage;
#[path = "live_merge_plan.rs"]
mod live_merge_plan;
#[path = "live_session_status.rs"]
mod live_session_status;
#[path = "pair_admission_bounded_max.rs"]
mod pair_admission_bounded_max;
#[path = "refactor_ast_access.rs"]
mod refactor_ast_access;
#[path = "refactor_consolidate.rs"]
mod refactor_consolidate;
#[path = "refactor_content_gate.rs"]
mod refactor_content_gate;
#[path = "refactor_extract.rs"]
mod refactor_extract;
#[path = "refactor_extract_negative.rs"]
mod refactor_extract_negative;
#[path = "refactor_extract_write_gate.rs"]
mod refactor_extract_write_gate;
#[path = "refactor_merge.rs"]
mod refactor_merge;
#[path = "refactor_merge_refusals.rs"]
mod refactor_merge_refusals;
#[path = "report_api.rs"]
mod report_api;
#[path = "report_fixture_file_identity.rs"]
mod report_fixture_file_identity;
