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

/// The HTTP mock for the embedding provider, declared once for the whole
/// suite. Twelve files used to `#[path]`-include it; as one binary that is
/// the same file loaded twelve times (`clippy::duplicate_mod`), so it is
/// declared here and reached as `crate::mock_ollama` everywhere.
#[path = "cli/mock_ollama.rs"]
mod mock_ollama;

#[path = "boilerplate.rs"]
mod boilerplate;
#[path = "cache_blob_integrity.rs"]
mod cache_blob_integrity;
#[path = "cache_key_lossy_utf8_collision.rs"]
mod cache_key_lossy_utf8_collision;
#[path = "cache_retention.rs"]
mod cache_retention;
#[path = "cli.rs"]
mod cli;
#[path = "config_include_dependencies.rs"]
mod config_include_dependencies;
#[path = "corpus_manifest_contract.rs"]
mod corpus_manifest_contract;
#[path = "cross_cluster_collapse.rs"]
mod cross_cluster_collapse;
#[path = "cross_cluster_enclosure.rs"]
mod cross_cluster_enclosure;
#[path = "cross_language.rs"]
mod cross_language;
#[path = "csharp_issue_66_route_mapping.rs"]
mod csharp_issue_66_route_mapping;
#[path = "csharp_merged_clone_families.rs"]
mod csharp_merged_clone_families;
#[path = "csharp_type1_type2_distinct_buckets.rs"]
mod csharp_type1_type2_distinct_buckets;
#[path = "csharp_unrelated_xunit_classes.rs"]
mod csharp_unrelated_xunit_classes;
#[path = "dart_forwarding_fail_open.rs"]
mod dart_forwarding_fail_open;
#[path = "dart_issue_119_embedding_role_mismatch.rs"]
mod dart_issue_119_embedding_role_mismatch;
#[path = "dart_issue_197_single_file_structural_only.rs"]
mod dart_issue_197_single_file_structural_only;
#[path = "dart_signatures.rs"]
mod dart_signatures;
#[path = "declaration_family_mixed_component.rs"]
mod declaration_family_mixed_component;
#[path = "declaration_family_plurality.rs"]
mod declaration_family_plurality;
#[path = "defaults.rs"]
mod defaults;
#[path = "diff_ingest_refusals.rs"]
mod diff_ingest_refusals;
#[path = "diff_scoped_ingest.rs"]
mod diff_scoped_ingest;
#[path = "diff_scoped_reporting.rs"]
mod diff_scoped_reporting;
#[path = "embedding_discovery_route.rs"]
mod embedding_discovery_route;
#[path = "embedding_non_finite.rs"]
mod embedding_non_finite;
#[path = "embedding_perf.rs"]
mod embedding_perf;
#[path = "embedding_route_invariance.rs"]
mod embedding_route_invariance;
#[path = "fsharp_deep_match_stack_overflow.rs"]
mod fsharp_deep_match_stack_overflow;
#[path = "fsharp_issue_336_data_table_category.rs"]
mod fsharp_issue_336_data_table_category;
#[path = "fsharp_issue_339_sibling_window_rename.rs"]
mod fsharp_issue_339_sibling_window_rename;
#[path = "fsharp_issue_339_token_fallback_rename.rs"]
mod fsharp_issue_339_token_fallback_rename;
#[path = "fused_golden_bands.rs"]
mod fused_golden_bands;
#[path = "fused_golden_invariants.rs"]
mod fused_golden_invariants;
#[path = "fused_score_bounds.rs"]
mod fused_score_bounds;
#[path = "go_vendor_exclusion.rs"]
mod go_vendor_exclusion;
#[path = "incremental_equivalence.rs"]
mod incremental_equivalence;
#[path = "incremental_multilang_golden.rs"]
mod incremental_multilang_golden;
#[path = "incremental_multilang_matrix.rs"]
mod incremental_multilang_matrix;
#[path = "issue_119_role_gate_exercised.rs"]
mod issue_119_role_gate_exercised;
#[path = "issue_132_subcommand_lookalike_path.rs"]
mod issue_132_subcommand_lookalike_path;
#[path = "issue_134_structural_only_not_nearly_identical.rs"]
mod issue_134_structural_only_not_nearly_identical;
#[path = "issue_165_dart_generated_header.rs"]
mod issue_165_dart_generated_header;
#[path = "issue_168_deep_nesting_no_crash.rs"]
mod issue_168_deep_nesting_no_crash;
#[path = "issue_169_dart_const_registry.rs"]
mod issue_169_dart_const_registry;
#[path = "issue_169_dart_filter_precision.rs"]
mod issue_169_dart_filter_precision;
#[path = "issue_190_data_table_demote.rs"]
mod issue_190_data_table_demote;
#[path = "issue_331_336_shape_only_saturation.rs"]
mod issue_331_336_shape_only_saturation;
#[path = "issue_342_scan_root_under_excluded_ancestor.rs"]
mod issue_342_scan_root_under_excluded_ancestor;
#[path = "issue_343_sum_clamp_saturation.rs"]
mod issue_343_sum_clamp_saturation;
#[path = "issue_362_two_file_const_tables.rs"]
mod issue_362_two_file_const_tables;
#[path = "issue_372_identical_snippet_cosine.rs"]
mod issue_372_identical_snippet_cosine;
#[path = "issue_389_subsumption_modifier_straddle.rs"]
mod issue_389_subsumption_modifier_straddle;
#[path = "js_language_features.rs"]
mod js_language_features;
#[path = "js_ts_clone_buckets.rs"]
mod js_ts_clone_buckets;
#[path = "js_ts_extensions.rs"]
mod js_ts_extensions;
#[path = "js_ts_false_positive_filters.rs"]
mod js_ts_false_positive_filters;
#[path = "js_ts_negative_controls.rs"]
mod js_ts_negative_controls;
#[path = "js_ts_normalization.rs"]
mod js_ts_normalization;
#[path = "js_ts_signatures.rs"]
mod js_ts_signatures;
#[path = "jsx_tsx_components.rs"]
mod jsx_tsx_components;
#[path = "jwt_independent_verification_false_positive.rs"]
mod jwt_independent_verification_false_positive;
#[path = "live_session_equivalence.rs"]
mod live_session_equivalence;
#[path = "location_rendering.rs"]
mod location_rendering;
#[path = "lsh_only_nearmiss_recall.rs"]
mod lsh_only_nearmiss_recall;
#[path = "metric_excludes_hidden_clusters.rs"]
mod metric_excludes_hidden_clusters;
#[path = "metric_language_agnostic.rs"]
mod metric_language_agnostic;
#[path = "metrics_folder_rollup.rs"]
mod metrics_folder_rollup;
#[path = "ollama_failures.rs"]
mod ollama_failures;
#[path = "operator_drift_is_not_duplication.rs"]
mod operator_drift_is_not_duplication;
#[path = "pair_size_coherence.rs"]
mod pair_size_coherence;
#[path = "polymorphic_gate_hides_rename_clone.rs"]
mod polymorphic_gate_hides_rename_clone;
#[path = "python_dict_assert_payload_proof.rs"]
mod python_dict_assert_payload_proof;
#[path = "python_dict_assert_reach.rs"]
mod python_dict_assert_reach;
#[path = "python_dict_assert_rhs_logic.rs"]
mod python_dict_assert_rhs_logic;
#[path = "python_dict_false_positive.rs"]
mod python_dict_false_positive;
#[path = "python_generated_template_false_positive.rs"]
mod python_generated_template_false_positive;
#[path = "python_inherited_contract_boundary.rs"]
mod python_inherited_contract_boundary;
#[path = "python_issue_100_kwargs_ctor.rs"]
mod python_issue_100_kwargs_ctor;
#[path = "python_issue_103_helper_call_sites.rs"]
mod python_issue_103_helper_call_sites;
#[path = "python_issue_104_module_preamble.rs"]
mod python_issue_104_module_preamble;
#[path = "python_issue_105_mapped_column.rs"]
mod python_issue_105_mapped_column;
#[path = "python_issue_107_chained_dict_assert.rs"]
mod python_issue_107_chained_dict_assert;
#[path = "python_issue_112_dict_fixture.rs"]
mod python_issue_112_dict_fixture;
#[path = "python_issue_115_pydantic_partial.rs"]
mod python_issue_115_pydantic_partial;
#[path = "python_issue_115_strenum.rs"]
mod python_issue_115_strenum;
#[path = "python_issue_119_embedding_role_mismatch.rs"]
mod python_issue_119_embedding_role_mismatch;
#[path = "python_issue_133_constant_table.rs"]
mod python_issue_133_constant_table;
#[path = "python_issue_69_abstract_method.rs"]
mod python_issue_69_abstract_method;
#[path = "python_issue_72_monkeypatch.rs"]
mod python_issue_72_monkeypatch;
#[path = "python_issue_96_all_exports.rs"]
mod python_issue_96_all_exports;
#[path = "python_issue_97_parametric_invariant_tests.rs"]
mod python_issue_97_parametric_invariant_tests;
#[path = "python_literal_variation_calls.rs"]
mod python_literal_variation_calls;
#[path = "python_same_shape_backends.rs"]
mod python_same_shape_backends;
#[path = "python_signatures.rs"]
mod python_signatures;
#[path = "rank_structural_only_policy.rs"]
mod rank_structural_only_policy;
#[path = "rename_literal_monotonicity.rs"]
mod rename_literal_monotonicity;
#[path = "rename_literal_substring_boundary.rs"]
mod rename_literal_substring_boundary;
#[path = "report_golden.rs"]
mod report_golden;
#[path = "rerun.rs"]
mod rerun;
#[path = "rust_issue_147_iter_collect_idiom.rs"]
mod rust_issue_147_iter_collect_idiom;
#[path = "rust_issue_150_mod_declarations.rs"]
mod rust_issue_150_mod_declarations;
#[path = "rust_issue_154_structural_only_signatures.rs"]
mod rust_issue_154_structural_only_signatures;
#[path = "rust_issue_176_match_dispatch.rs"]
mod rust_issue_176_match_dispatch;
#[path = "rust_issue_224_struct_field_runs.rs"]
mod rust_issue_224_struct_field_runs;
#[path = "rust_issue_232_token_jaccard_identical.rs"]
mod rust_issue_232_token_jaccard_identical;
#[path = "rust_test_boilerplate_false_positive.rs"]
mod rust_test_boilerplate_false_positive;
#[path = "rust_trait_boilerplate_false_positive.rs"]
mod rust_trait_boilerplate_false_positive;
#[path = "showstoppers.rs"]
mod showstoppers;
#[path = "sibling_dedup.rs"]
mod sibling_dedup;
#[path = "sibling_ranking.rs"]
mod sibling_ranking;
#[path = "signature_reuse.rs"]
mod signature_reuse;
#[path = "skip_policy_contract.rs"]
mod skip_policy_contract;
#[path = "ts_issue_283_object_literal_tables.rs"]
mod ts_issue_283_object_literal_tables;
#[path = "ts_issue_284_produce_then_assert.rs"]
mod ts_issue_284_produce_then_assert;
#[path = "ts_issue_285_diagnostic_scenarios.rs"]
mod ts_issue_285_diagnostic_scenarios;
#[path = "type2_rename_anchor_floor.rs"]
mod type2_rename_anchor_floor;
#[path = "type3_enclosing_method.rs"]
mod type3_enclosing_method;
#[path = "typescript_features.rs"]
mod typescript_features;
#[path = "verbatim_subgroup_survives_noise.rs"]
mod verbatim_subgroup_survives_noise;
