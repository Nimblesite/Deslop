# Token signatures

## [PIPELINE-SIGNATURE-FOLD] Construct signatures once per parsed tree

The token signature pass folds each normalized tree from its leaves upward. A parent combines its children's MinHash slots and the k-grams crossing their boundaries; sibling windows use the same composition. The result must match the reference top-down token construction byte for byte. Parsed-file signatures are persisted with the fingerprints and consumed by the render pass. Explicit cross-language signatures remain a separate render-time projection under [CONFIG-CROSS-LANGUAGE].

`crates/deslop-core/src/pipeline/signatures.rs` and its `fold` module implement the fold. `fold_signatures_match_the_top_down_construction` checks equivalence to the reference construction.

## [PIPELINE-SIGNATURE-FALLBACK] A missing token signature stays fingerprint-scoped

When a range cannot resolve to tokens, or the token stream is shorter than the configured k-gram width, derive the fallback from the fingerprint hash and byte range. Unrelated empty or too-short streams must not share a signature merely because both have no k-grams. Hash offsets as fixed-width little-endian values so persisted signatures do not depend on the host architecture.

`fallback_signature` in `crates/deslop-core/src/pipeline/signatures.rs` implements this rule. `too_short_streams_stay_fingerprint_scoped`, `issue_86_unresolvable_ranges_are_fingerprint_scoped`, and `fallback_signature_slots_are_architecture_independent` in its `tests.rs` pin distinct ranges, unresolved views, and stable slot bytes.
