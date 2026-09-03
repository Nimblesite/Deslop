//! Unit pins for [CLONE-NOISE-LITERAL-VARIATION-CALLS] at the
//! `is_noise_pattern` seam: member count is not a suppression
//! clause, so a two-member family must be judged on its call shapes —
//! convicted when its variation is plain payload, published when the
//! variation is authored interpolation (gh #467) or byte-identical.

use super::super::{is_noise_pattern, NoiseFilter, ParseCache};
use crate::ast::ByteRange;
use crate::fingerprint::Fingerprint;
use crate::state::{FileId, FileRegistry};

use std::collections::HashMap;
use std::path::PathBuf;

/// One member's source: an invariant call over a varying plain string.
const PLAIN_A: &str = "def member_a():\n    greet(\"alice\")\n";
/// The second member: same callee, same arity, different plain string.
const PLAIN_B: &str = "def member_b():\n    greet(\"bob\")\n";

/// Two members whose only difference is an authored interpolation: the
/// variation is code choosing data (gh #467), so the pair publishes.
const INTERP_A: &str = "def member_a():\n    describe(f\"user-{index}\")\n";
/// The interpolation counterpart.
const INTERP_B: &str = "def member_b():\n    describe(f\"order-{index}\")\n";

/// Two members with byte-identical literals: a copy, never scaffolding.
const SAME_A: &str = "def member_a():\n    greet(\"alice\")\n";
/// The byte-identical counterpart.
const SAME_B: &str = "def member_a():\n    greet(\"alice\")\n";

/// gh #284 whole-scenario members: an invariant adapter whose bound
/// result flows into the varying assertion through its receiver.
const RECEIVER_FLOW_A: &str = "test(\"points an optional empty record at the null offset\", () => {\n  const schema = loadFixture(\"empty-record-optional.td\");\n  const generated = generateRust(schema, { tdbin: true });\n  expect(generated).toContain(\"pub marker: Option<EmptyMarker>\");\n});\n";
/// The receiver-flow counterpart.
const RECEIVER_FLOW_B: &str = "test(\"points a required empty record at the shared singleton\", () => {\n  const schema = loadFixture(\"empty-record-required.td\");\n  const generated = generateRust(schema, { tdbin: true });\n  expect(generated).toContain(\"pub marker: EmptyMarker\");\n});\n";
/// The needle locating the adapter call.
const ADAPTER_NEEDLE: &str = "generateRust(";

/// The needle locating the call statement inside a member source.
const CALL_NEEDLE: &str = "greet(";
/// The needle locating the interpolation call.
const INTERP_NEEDLE: &str = "describe(";

/// A plain-literal two-member pair at the Split stage is exactly the
/// scaffolding shape the filter names: same callee and arity, every
/// literal-bearing position differing, so the family must be convicted.
#[test]
fn two_member_plain_literal_pair_is_convicted_at_split() {
    let verdict = Corpus::new()
        .member(PLAIN_A, CALL_NEEDLE)
        .member(PLAIN_B, CALL_NEEDLE)
        .verdict();
    assert_eq!(
        verdict,
        Some(NoiseFilter::LiteralCalls),
        "a two-member family varying one plain string literal over a \
         shared callee is [CLONE-NOISE-LITERAL-VARIATION-CALLS] \
         scaffolding and must be convicted at the Split stage: {verdict:?}"
    );
}

/// An interpolation-varying pair is a copy-pasted pair, not a family to
/// parameterise — the filter must decline and let it publish (gh #467).
#[test]
fn interpolation_varying_pair_publishes_at_split() {
    let verdict = Corpus::new()
        .member(INTERP_A, INTERP_NEEDLE)
        .member(INTERP_B, INTERP_NEEDLE)
        .verdict();
    assert_eq!(
        verdict, None,
        "the differing argument is authored interpolation — code choosing \
         data — so the pair is a copy-paste pair and must publish: {verdict:?}"
    );
}

/// Byte-identical members never match the variation rule: a copy keeps
/// the family's verbatim escape hatch.
#[test]
fn byte_identical_call_pair_is_not_convicted() {
    let verdict = Corpus::new()
        .member(SAME_A, CALL_NEEDLE)
        .member(SAME_B, CALL_NEEDLE)
        .verdict();
    assert_eq!(
        verdict, None,
        "members whose literals all agree are a byte-identical copy and \
         must not be convicted by the literal-variation filter: {verdict:?}"
    );
}

/// The same gh #285 pair through the whole noise bank: the Split-stage
/// veto must not refuse a two-member family whose call shapes name the
/// scaffolding, and the Render stage must convict it.
#[test]
fn ts_scenario_pair_verdict_through_the_noise_bank() {
    let block_a = "test(\"rejects an unsupported field type\", () => {\n  const schema = buildSchema({ kind: \"record\", fields: [{ name: \"at\", type: \"Duration\" }] });\n  const result = encodeTdbin(schema);\n  expectErrorMessages(result, [\"field type is not supported by the binary codec\"]);\n});\n";
    let block_b = "test(\"rejects a typed map key\", () => {\n  const schema = buildSchema({ kind: \"record\", fields: [{ name: \"index\", type: \"Map<Point, i32>\" }] });\n  const result = encodeTdbin(schema);\n  expectErrorMessages(result, [\"typed map keys must be scalars\"]);\n});\n";
    let corpus = Corpus::new()
        .member(block_a, "encodeTdbin")
        .member(block_b, "encodeTdbin")
        .language("typescript");
    let verdict = corpus.verdict();
    assert_eq!(
        verdict,
        Some(NoiseFilter::LiteralCalls),
        "the two-member scenario family must be convicted through the \
         full noise bank: {verdict:?}"
    );
}

/// The gh #284 pair whose invariant adapter is consumed through the
/// *receiver* of the varying call: `generateRust` binds `generated`,
/// and `expect(generated).toContain("…")` reads it as the subject of
/// the assertion rather than as an argument. The spec's adapter clause
/// says a bound result that flows into a later varying call is
/// connective plumbing, and a receiver is one way a value flows in, so
/// the whole-scenario pair must be convicted.
#[test]
fn adapter_consumed_through_the_receiver_is_still_scaffolding() {
    let corpus = Corpus::new()
        .member(RECEIVER_FLOW_A, ADAPTER_NEEDLE)
        .member(RECEIVER_FLOW_B, ADAPTER_NEEDLE)
        .language("typescript");
    let verdict = corpus.verdict();
    assert_eq!(
        verdict,
        Some(NoiseFilter::LiteralCalls),
        "the invariant `generateRust` adapter binds `generated`, which the \
         varying `expect(generated).toContain(…)` consumes as its receiver; \
         a value flowing into the callee is still flowing into the call, so \
         the scenario pair is literal-variation scaffolding: {verdict:?}"
    );
}

/// The member corpus: registered sources plus one whole-member
/// fingerprint apiece, mirroring the whole-filter harness the
/// polymorphic and dict-assert pins use.
struct Corpus {
    registry: FileRegistry,
    sources: Vec<(FileId, &'static str)>,
    members: Vec<Fingerprint>,
    language: &'static str,
}

impl Corpus {
    fn new() -> Self {
        Self {
            registry: FileRegistry::new(),
            sources: Vec::new(),
            members: Vec::new(),
            language: "python",
        }
    }

    /// Overrides the harness language for multi-language pins.
    fn language(mut self, language: &'static str) -> Self {
        self.language = language;
        self
    }

    /// Registers `source` and offers the range from the file start
    /// through the end of the call statement as one member.
    fn member(mut self, source: &'static str, needle: &str) -> Self {
        assert!(
            source.contains(needle),
            "fixture needle {needle:?} must exist in the member source"
        );
        let file_id = self.registry.register(PathBuf::from("src.py"));
        let call_at = source.find(needle).unwrap_or_default();
        let statement_end = source.len().saturating_sub(1);
        self.sources.push((file_id, source));
        self.members.push(Fingerprint {
            hash: [0_u8; 32],
            file_id,
            byte_range: ByteRange {
                start: 0,
                end: statement_end.max(call_at),
            },
            node_count: 12,
        });
        self
    }

    /// Runs the whole noise bank over the members.
    fn verdict(self) -> Option<NoiseFilter> {
        let sources: HashMap<FileId, Vec<u8>> = self
            .sources
            .iter()
            .map(|(file_id, text)| (*file_id, text.as_bytes().to_vec()))
            .collect();
        let languages: HashMap<FileId, &'static str> = self
            .sources
            .iter()
            .map(|(file_id, _)| (*file_id, self.language))
            .collect();
        is_noise_pattern(&self.members, &sources, &languages, &ParseCache::new())
    }
}
