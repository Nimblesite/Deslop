//! Unit pins for [CLONE-NOISE-LITERAL-VARIATION-CALLS] at the
//! `is_noise_pattern` seam: member count is not a suppression
//! clause, so a two-member family must be judged on its call shapes —
//! convicted when its variation is plain payload, published when the
//! variation is authored interpolation (gh #467) or byte-identical.

use std::{collections::HashMap, path::PathBuf};

use super::super::{is_noise_pattern, NoiseFilter, ParseCache};
use crate::{
    ast::ByteRange,
    fingerprint::Fingerprint,
    state::{FileId, FileRegistry},
};

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

/// Two TypeScript test cases with byte-identical multi-statement bodies
/// and differing names: the bodies are the duplicate, the names are not
/// payload, so the family must publish.
const TS_TEST_A: &str = "it('renders the header', () => {\n  const view = mount(Header);\n  expect(view.text()).toContain('Deslop');\n  expect(view.find('nav').length).toBe(1);\n});\n";
/// The counterpart, differing only in the test name.
const TS_TEST_B: &str = "it('renders the footer', () => {\n  const view = mount(Header);\n  expect(view.text()).toContain('Deslop');\n  expect(view.find('nav').length).toBe(1);\n});\n";
/// The needle locating the test-case call.
const TS_TEST_NEEDLE: &str = "it(";

/// Two Dart `group` wrappers, each holding one identical test body, whose
/// only difference is the group name.
const DART_GROUP_A: &str = "group('upload', () {\n  test('supports progress', () {\n    final bloc = CounterBloc();\n    expect(bloc.state, equals(0));\n  });\n});\n";
/// The counterpart, differing only in the group name.
const DART_GROUP_B: &str = "group('download', () {\n  test('supports progress', () {\n    final bloc = CounterBloc();\n    expect(bloc.state, equals(0));\n  });\n});\n";
/// The needle locating the group call.
const DART_GROUP_NEEDLE: &str = "group(";

/// A call carrying a statement-bearing argument is judged by that body,
/// never by the string literal beside it: two test cases whose bodies
/// are copies are a clone, whatever their names say
/// ([CLONE-NOISE-LITERAL-VARIATION-CALLS]).
#[test]
fn test_bodies_are_judged_by_their_statements_not_their_names() {
    let verdict = Corpus::new()
        .call_member(TS_TEST_A, TS_TEST_NEEDLE)
        .call_member(TS_TEST_B, TS_TEST_NEEDLE)
        .language("typescript")
        .verdict();
    assert_eq!(
        verdict, None,
        "the members' bodies are byte-identical statements, so the differing \
         test name is not literal-variation payload: {verdict:?}"
    );
}

/// The same rule one level up: a `group` wrapper carries a body, so its
/// name is not payload and the wrapper pair must publish.
#[test]
fn dart_group_wrappers_carry_bodies_so_their_names_are_not_payload() {
    let verdict = Corpus::new()
        .call_member(DART_GROUP_A, DART_GROUP_NEEDLE)
        .call_member(DART_GROUP_B, DART_GROUP_NEEDLE)
        .language("dart")
        .verdict();
    assert_eq!(
        verdict, None,
        "a wrapper whose argument carries statements is authored logic, \
         never scaffolding varying a literal: {verdict:?}"
    );
}

/// One browser check inside a test body: the member is the awaited
/// `locator().boundingBox()` expression, not the test case around it.
const AWAITED_LOCATOR_A: &str = "test(\"the header sits above the sidebar\", async ({ page }) => {\n  const headerBox = await page.locator(\".site-header\").boundingBox();\n  expect(headerBox.y).toBe(0);\n});\n";
/// A second check on another selector, awaited the same way.
const AWAITED_LOCATOR_B: &str = "test(\"the stamp sits above the filters\", async ({ page }) => {\n  const stampBox = await page.locator(\".atlas-publication\").boundingBox();\n  expect(stampBox.y).toBe(0);\n});\n";
/// The needle opening each awaited expression.
const AWAITED_NEEDLE: &str = "await page";
/// The text closing each awaited expression.
const AWAITED_CLOSE: &str = ".boundingBox()";

/// The same awaited check inside a plain helper, with no call around it
/// at all.
const AWAITED_IN_HELPER_A: &str = "export async function checkHeader(page) {\n  const headerBox = await page.locator(\".site-header\").boundingBox();\n  expect(headerBox.y).toBe(0);\n}\n";
/// A second helper on another selector, awaited the same way.
const AWAITED_IN_HELPER_B: &str = "export async function checkStamp(page) {\n  const stampBox = await page.locator(\".atlas-publication\").boundingBox();\n  expect(stampBox.y).toBe(0);\n}\n";

/// A typed decorator's overload block: a type variable bound by one call,
/// then overload signatures that carry no call at all.
const OVERLOADS_COMMAND: &str = "CmdType = t.TypeVar(\"CmdType\", bound=Command)\n\n@t.overload\ndef command(name: _AnyCallable) -> Command: ...\n\n@t.overload\ndef command(name: str | None, cls: type[CmdType], **attrs: t.Any) -> t.Callable[[_AnyCallable], CmdType]: ...\n";
/// The same block for the group decorator — the systematic rename
/// `command -> group`, `CmdType -> GrpType`, and nothing else.
const OVERLOADS_GROUP: &str = "GrpType = t.TypeVar(\"GrpType\", bound=Group)\n\n@t.overload\ndef group(name: _AnyCallable) -> Group: ...\n\n@t.overload\ndef group(name: str | None, cls: type[GrpType], **attrs: t.Any) -> t.Callable[[_AnyCallable], GrpType]: ...\n";
/// The needle opening each overload block.
const OVERLOADS_NEEDLE: &str = "t.TypeVar(";

/// A run of statements that happens to contain exactly one call is not
/// an expression around that call: the `click` overload blocks are one
/// code renamed throughout, and judging them by the one `TypeVar` call
/// they contain — whose string is the type variable's own name — hid
/// the copy. A member holding a complete statement keeps the sequence
/// rule, which refuses to convict a run of call-free definitions
/// ([CLONE-NOISE-LITERAL-VARIATION-CALLS-MEMBER-CALL],
/// [CLONE-NOISE-LITERAL-VARIATION-CALLS-COVERED-STATEMENT]).
#[test]
fn a_statement_run_is_never_judged_by_the_one_call_it_contains() {
    let verdict = Corpus::new()
        .member(OVERLOADS_COMMAND, OVERLOADS_NEEDLE)
        .member(OVERLOADS_GROUP, OVERLOADS_NEEDLE)
        .verdict();
    assert_eq!(
        verdict, None,
        "the members are runs of overload definitions renamed throughout, not one call \
         varying its literal: {verdict:?}"
    );
}

/// A member that is the `await` of one call and has no enclosing call
/// at all — the check sits in a plain function body — is judged by the
/// one call it holds exactly as one inside a test body is
/// ([CLONE-NOISE-LITERAL-VARIATION-CALLS-MEMBER-CALL]).
#[test]
fn an_awaited_call_with_no_enclosing_call_is_judged_by_its_own_call() {
    let verdict = Corpus::new()
        .expression_member(AWAITED_IN_HELPER_A, AWAITED_NEEDLE, AWAITED_CLOSE)
        .expression_member(AWAITED_IN_HELPER_B, AWAITED_NEEDLE, AWAITED_CLOSE)
        .language("typescript")
        .verdict();
    assert_eq!(
        verdict,
        Some(NoiseFilter::LiteralCalls),
        "no call encloses the awaited expression, so nothing but the call it holds can \
         judge it, and that call varies only its selector: {verdict:?}"
    );
}

/// A member that is the `await` of one call sits inside a test body, so
/// the smallest call enclosing it carries that body and may not judge
/// it; the member is judged by the one call it holds, whose selector
/// varies ([CLONE-NOISE-LITERAL-VARIATION-CALLS-MEMBER-CALL]).
#[test]
fn an_awaited_call_inside_a_test_body_is_judged_by_its_own_call() {
    let verdict = Corpus::new()
        .expression_member(AWAITED_LOCATOR_A, AWAITED_NEEDLE, AWAITED_CLOSE)
        .expression_member(AWAITED_LOCATOR_B, AWAITED_NEEDLE, AWAITED_CLOSE)
        .language("typescript")
        .verdict();
    assert_eq!(
        verdict,
        Some(NoiseFilter::LiteralCalls),
        "the awaited call is `page.locator(SELECTOR).boundingBox()` in both members and \
         only the selector differs; the test case around it is not the member: {verdict:?}"
    );
}

impl Corpus {
    /// Registers `source` and offers the expression running from `needle`
    /// through the end of `close` as one member — a member that is not a
    /// whole statement and not a whole call.
    fn expression_member(mut self, source: &'static str, needle: &str, close: &str) -> Self {
        assert!(
            source.contains(needle) && source.contains(close),
            "fixture needle {needle:?} and close {close:?} must exist in the member source"
        );
        let start = source.find(needle).unwrap_or_default();
        let end = source
            .find(close)
            .map_or(source.len(), |at| at.saturating_add(close.len()));
        let file_id = self.registry.register(PathBuf::from("src.py"));
        self.sources.push((file_id, source));
        self.members.push(Fingerprint {
            hash: [0_u8; 32],
            file_id,
            byte_range: ByteRange { start, end },
            node_count: 12,
        });
        self
    }

    /// Registers `source` and offers exactly the call that starts at
    /// `needle` and ends at the source's last closing parenthesis as one
    /// member, so the filter judges the call itself.
    fn call_member(mut self, source: &'static str, needle: &str) -> Self {
        assert!(
            source.contains(needle) && source.contains(')'),
            "fixture needle {needle:?} and a closing parenthesis must exist in the member source"
        );
        let start = source.find(needle).unwrap_or_default();
        let end = source
            .rfind(')')
            .map_or(source.len(), |close| close.saturating_add(1));
        let file_id = self.registry.register(PathBuf::from("src.py"));
        self.sources.push((file_id, source));
        self.members.push(Fingerprint {
            hash: [0_u8; 32],
            file_id,
            byte_range: ByteRange { start, end },
            node_count: 12,
        });
        self
    }
}

/// The gh #284 shape across two producers: a Rust scenario and a
/// TypeScript scenario share the produce-then-assert idiom, and the only
/// literal-free position — the producer whose result the varying
/// assertions consume — names a different helper in each.
const PRODUCER_RUST: &str = "test(\"documents the generated Rust module\", () => {\n  const schema = loadFixture(\"documented-records.td\");\n  const generated = generateRust(schema, { tdbin: true });\n  expect(generated).toContain(\"pub struct Customer {\");\n});\n";
/// The TypeScript counterpart, produced by a different helper.
const PRODUCER_TYPESCRIPT: &str = "test(\"generates the Option scalar layout\", () => {\n  const schema = loadFixture(\"option-scalars.td\");\n  const generated = generateTypeScript(schema, { tdbin: true });\n  expect(generated).toContain(\"export type OptionScalars = {\");\n});\n";
/// The needle locating each scenario's producer call.
const PRODUCER_NEEDLE: &str = "loadFixture(";

/// A literal-free adapter may name a different helper in each member:
/// the scaffold is the produce-then-assert idiom, not the producer.
#[test]
fn a_renamed_literal_free_adapter_keeps_the_scenario_family_convicted() {
    let verdict = Corpus::new()
        .member(PRODUCER_RUST, PRODUCER_NEEDLE)
        .member(PRODUCER_TYPESCRIPT, PRODUCER_NEEDLE)
        .language("typescript")
        .verdict();
    assert_eq!(
        verdict,
        Some(NoiseFilter::LiteralCalls),
        "the producer position carries no literal and its result flows into \
         the varying assertion, so a different producer name is plumbing, \
         not logic: {verdict:?}"
    );
}
