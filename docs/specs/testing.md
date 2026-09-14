# Test suite structure

## [TEST-ONE-BINARY] Link each crate's integration suite once

The `deslop`, `deslop-core`, `deslop-lsp`, and `deslop-mcp` crates disable Cargo's automatic integration-test discovery and explicitly register their `tests/suite.rs` entry point. That entry point declares the component suites as modules. Every suite remains compiled and selectable; collecting modules must not drop tests or create a second automatically discovered executable for each source file.

The four crate manifests and suite entry points implement this build contract. `crates/deslop/tests/skip_policy_contract.rs` checks that every corpus Makefile command names a test target the `deslop` manifest actually declares. This is a test-build rule; it adds no production runtime behavior. Release profile and CI consumers follow [CI-RELEASE-BUILD].

## [SPEC-ID-GATE] A cited rule identifier must resolve to a written rule

Comments cite a rule by its bracketed identifier so that searching for that identifier finds three things: the written rule, the code implementing it, and the tests holding it. The trace is the project's way of keeping documents, code and tests honest with one another, and it fails silently — searching an invented identifier returns the code and the tests and simply nothing else, which reads exactly like a rule nobody happened to link.

Thirty-six identifiers had drifted that way. Twenty were cited from shipped code, one was the worked example the contributor instructions use, and one kept two extension commands deleted under a rule no document stated. Nothing was wrong with any individual comment; there was no gate, so the trace decayed a citation at a time.

**Every identifier cited by a comment under `crates/` or `clients/vscode/src` resolves to a document under `docs/`.** The gate is `crates/deslop/tests/spec_id_traceability.rs`, so it runs in the ordinary test job and a newly invented identifier cannot merge without the rule it points at. Three resolutions are acceptable and no others: give the identifier a section carrying it, rename the citation to the identifier it actually meant, or delete the citation together with the code it describes.

Citations are read from **parsed comment nodes**, never from a text scan of source, so an identifier appearing in a string literal or a fixture is not a citation. Documents are prose: a heading defines every identifier it carries, wherever in the heading it sits, and any other line defines an identifier only when that identifier leads it, after list markers, ordinals and bold markers. A fenced block defines nothing — code samples quote rules, they do not state them.

`scripts/repository/spec-crossrefs.py` reads the same two indexes to render the human audit with links; the gate is what enforces them.
