//! [CORPUS-PRECISION] Both directions of the `must_not_rank_first`
//! predicate (gh #401).
//!
//! The shipped rule was `occurrence_text.contains("extends StatefulWidget")`
//! — text pattern matching on source code, which `CLAUDE.md` prohibits
//! outright, and which is wrong in both directions at once. These tests
//! state the two directions as assertions so neither can come back:
//!
//! * a *mention* of the supertype — in a comment, a doc comment, a string
//!   literal, or a constructor call in the body — is not a declaration, so
//!   the gate must not report the cluster as boilerplate;
//! * a declaration whose `extends` clause is spaced or wrapped differently
//!   is the same declaration, so the gate must report it.

use anyhow::{anyhow, Result};

/// [CORPUS-PRECISION] The language a ranked cluster is judged in.
mod language_of_first_occurrence;

use super::declares_forbidden_supertype;
use crate::enclosure::Span;

const DART_LANGUAGE: &str = "dart";
const STATEFUL_WIDGET: &str = "StatefulWidget";
const STATELESS_WIDGET: &str = "StatelessWidget";

/// Judges `source` in full, as `language`, against `supertype`.
///
/// # Errors
///
/// Propagates the predicate's own error — an unregistered language, a
/// span outside the source, or a parse failure. Tests take it with `?`
/// rather than `expect`, so a broken fixture fails by name instead of
/// through a panic the workspace lint denies.
fn declares(language: &str, source: &str, supertype: &str) -> Result<bool> {
    let span = Span::new(
        "lib/widget.dart",
        0,
        u64::try_from(source.len()).unwrap_or(0),
    );
    declares_forbidden_supertype(language, source, &span, supertype)
}

/// Judges the sub-range of `source` delimited by `marker` .. end of source.
///
/// # Errors
///
/// Errors when `marker` is absent from the fixture — a fixture edit that
/// drops the marker would otherwise silently judge the whole source —
/// and propagates the predicate's own error.
fn declares_from(language: &str, source: &str, marker: &str, supertype: &str) -> Result<bool> {
    let start = source
        .find(marker)
        .ok_or_else(|| anyhow!("marker {marker:?} must exist in the fixture"))?;
    let span = Span::new(
        "lib/widget.dart",
        u64::try_from(start).unwrap_or(0),
        u64::try_from(source.len()).unwrap_or(0),
    );
    declares_forbidden_supertype(language, source, &span, supertype)
}

/// A Dart widget that mentions both forbidden supertypes everywhere except
/// where it would matter: a doc comment, a line comment, a string literal,
/// and a constructor call in the body. It extends `State`, not either of
/// them.
const MENTIONS_ONLY: &str = r"
/// A controller for a widget that extends StatefulWidget.
///
/// Prefer this over a class that extends StatelessWidget.
class LedgerController extends State<LedgerView> {
  // The parent extends StatefulWidget, which is why this exists.
  static const String kind = 'extends StatefulWidget';

  @override
  Widget build(BuildContext context) {
    debugPrint('extends StatelessWidget');
    return const SizedBox.shrink();
  }
}
";

#[test]
fn a_supertype_only_mentioned_in_comments_and_literals_is_not_a_declaration() -> Result<()> {
    assert!(
        !declares(DART_LANGUAGE, MENTIONS_ONLY, STATEFUL_WIDGET)?,
        "a doc comment, a line comment and a string literal all say \
         `extends StatefulWidget`; none of them declares it, and reporting \
         this cluster as framework boilerplate would suppress a real \
         finding on the strength of prose"
    );
    assert!(
        !declares(DART_LANGUAGE, MENTIONS_ONLY, STATELESS_WIDGET)?,
        "same in the other direction: `debugPrint('extends StatelessWidget')` \
         is an argument, not a base type"
    );
    assert!(
        declares(DART_LANGUAGE, MENTIONS_ONLY, "State")?,
        "the class really does extend `State`, so the predicate must be able \
         to see the one declaration that is actually there — otherwise this \
         fixture would pass by seeing nothing at all"
    );
    Ok(())
}

/// The same declaration, wrapped across lines the way a formatter wraps a
/// long generic header. Byte-for-byte this shares no `extends StatefulWidget`
/// substring with the flat spelling.
const WRAPPED_DECLARATION: &str = r"
class LedgerView
    extends
        StatefulWidget {
  const LedgerView({super.key});

  @override
  State<LedgerView> createState() => _LedgerViewState();
}
";

#[test]
fn a_wrapped_extends_clause_is_the_same_declaration() -> Result<()> {
    assert!(
        declares(DART_LANGUAGE, WRAPPED_DECLARATION, STATEFUL_WIDGET)?,
        "`extends` and `StatefulWidget` are on different lines, so no \
         substring of the source spells `extends StatefulWidget` — the \
         declaration is identical and the gate must still see it"
    );
    assert!(
        !declares(DART_LANGUAGE, WRAPPED_DECLARATION, STATELESS_WIDGET)?,
        "the wrapped clause names one supertype; the other must not be \
         inferred from it"
    );
    Ok(())
}

/// The flat spelling, which the substring rule did match. Kept so the fix
/// cannot be mistaken for a rule that stopped firing.
const FLAT_DECLARATION: &str = r"
class LedgerTile extends StatelessWidget {
  const LedgerTile({super.key});

  @override
  Widget build(BuildContext context) => const SizedBox.shrink();
}
";

#[test]
fn the_flat_declaration_still_fires_and_only_for_its_own_supertype() -> Result<()> {
    assert!(
        declares(DART_LANGUAGE, FLAT_DECLARATION, STATELESS_WIDGET)?,
        "the ordinary spelling is what gh #331 is about; it must keep firing"
    );
    assert!(
        !declares(DART_LANGUAGE, FLAT_DECLARATION, STATEFUL_WIDGET)?,
        "a StatelessWidget is not a StatefulWidget — a predicate that fired \
         on both would make the rule unfalsifiable"
    );
    Ok(())
}

#[test]
fn a_member_of_the_declaration_is_judged_by_its_enclosing_class() -> Result<()> {
    assert!(
        declares_from(
            DART_LANGUAGE,
            FLAT_DECLARATION,
            "  @override",
            STATELESS_WIDGET
        )?,
        "the ranked occurrence is usually the mandated member — Flutter's \
         `build`, or `createState` — not the class header. Judging only the \
         reported bytes would miss every cluster the rule exists to catch"
    );
    Ok(())
}

#[test]
fn an_unregistered_language_fails_the_gate_rather_than_passing_it() {
    let span = Span::new("Ledger.fs", 0, 0);
    let verdict = declares_forbidden_supertype("fsharp", "", &span, STATEFUL_WIDGET);
    assert!(
        verdict.is_err(),
        "a manifest naming a language this module has no heritage grammar \
         for must fail loudly. Returning `false` would switch the precision \
         gate off silently, which is the [CORPUS-BASELINE] failure the \
         ratchet then reads as evidence the defect is absent"
    );
}

/// One declaration per language in [`super::HERITAGE`], each paired with a
/// mention of a *different* framework type in a comment or a call, so a
/// language whose clause kinds are wrong cannot pass by matching prose.
const HERITAGE_CASES: [(&str, &str, &str, &str); 6] = [
    (
        "dart",
        "// LedgerTile is not a StatelessWidget.\nclass LedgerTile extends StatefulWidget {}\n",
        "StatefulWidget",
        "StatelessWidget",
    ),
    (
        "csharp",
        "// Not a PageModel.\npublic class LedgerPage : ControllerBase { }\n",
        "ControllerBase",
        "PageModel",
    ),
    (
        "typescript",
        "// Not a BaseEntity.\nexport class LedgerService extends AbstractService {}\n",
        "AbstractService",
        "BaseEntity",
    ),
    (
        "javascript",
        "// Not a HTMLElement.\nclass LedgerView extends Component {}\n",
        "Component",
        "HTMLElement",
    ),
    (
        "python",
        "# Not a BaseModel.\nclass LedgerView(APIView):\n    pass\n",
        "APIView",
        "BaseModel",
    ),
    (
        "php",
        "<?php\n// Not an Eloquent Model.\nclass LedgerController extends Controller {}\n",
        "Controller",
        "Model",
    ),
];

#[test]
fn every_curated_heritage_grammar_reads_its_own_base_clause() -> Result<()> {
    for (language, source, declared, mentioned) in HERITAGE_CASES {
        assert!(
            declares(language, source, declared)?,
            "{language}: the declaration names `{declared}` as its base type, \
             so the clause kinds curated for this language must find it — an \
             entry that cannot fire is a precision gate switched off"
        );
        assert!(
            !declares(language, source, mentioned)?,
            "{language}: `{mentioned}` appears only in a comment, and a \
             comment is not a base type"
        );
    }
    Ok(())
}

#[test]
fn a_type_argument_is_not_a_base_type() -> Result<()> {
    let source = "class LedgerViewState extends State<LedgerView> {}\n";
    assert!(
        declares(DART_LANGUAGE, source, "State")?,
        "`State` is the declared base type"
    );
    assert!(
        !declares(DART_LANGUAGE, source, "LedgerView")?,
        "`LedgerView` is what `State` was instantiated with, not a base type \
         — matching it would let one manifest entry condemn every widget in \
         the repository"
    );
    Ok(())
}

/// The same base-type-versus-type-argument distinction in the two grammars
/// that spell it differently from Dart: C# nests arguments under
/// `type_argument_list`, Python has no argument node at all and puts them in
/// a `subscript` field.
const GENERIC_BASE_CASES: [(&str, &str, &str, &str); 3] = [
    (
        "csharp",
        "public class LedgerPage : PageModel<LedgerEntry> { }\n",
        "PageModel",
        "LedgerEntry",
    ),
    (
        "typescript",
        "export class LedgerService extends AbstractService<LedgerEntry> {}\n",
        "AbstractService",
        "LedgerEntry",
    ),
    (
        "python",
        "class LedgerView(GenericAPIView[LedgerEntry]):\n    pass\n",
        "GenericAPIView",
        "LedgerEntry",
    ),
];

#[test]
fn a_type_argument_is_not_a_base_type_in_any_curated_grammar() -> Result<()> {
    for (language, source, base, argument) in GENERIC_BASE_CASES {
        assert!(
            declares(language, source, base)?,
            "{language}: `{base}` is the declared base type"
        );
        assert!(
            !declares(language, source, argument)?,
            "{language}: `{argument}` is a type argument, not a base type"
        );
    }
    Ok(())
}

#[test]
fn a_qualified_base_type_is_named_by_its_last_segment() -> Result<()> {
    assert!(
        declares(
            "javascript",
            "class LedgerView extends React.Component {}\n",
            "Component"
        )?,
        "`extends React.Component` extends `Component`; a rule naming the \
         type must not be defeated by the namespace it was reached through"
    );
    Ok(())
}

/// [CORPUS-PRECISION-CURATED] `precision` — a curated non-duplicate must
/// never be shown as one cluster.
mod curated_precision {
    use serde_json::{json, Value};

    use super::super::check_curated_precision;
    use crate::corpus::Failure;

    /// The pair every case below curates as *not* duplication.
    const PAIR: [&str; 2] = ["tests/test_configs.py", "tests/test_openapi.py"];

    /// A manifest curating one hand-verified non-duplicate over `files`.
    fn manifest(files: &[&str]) -> Value {
        json!({
            "must_not_cluster": [{
                "files": files,
                "why": "two unrelated pytest modules sharing only the assertion idiom",
                "verified": "read both functions; they assert different endpoints",
            }]
        })
    }

    /// One cluster over `files`, each occurrence hidden or not.
    fn cluster(files: &[&str], hidden: bool) -> Value {
        json!({
            "id": "c0ffee",
            "bucket": "nearly_identical",
            "size": files.len(),
            "signals": { "fused": 0.91 },
            "occurrences": files
                .iter()
                .map(|file| json!({ "path": file, "hidden": hidden }))
                .collect::<Vec<Value>>(),
        })
    }

    /// The check ids a run produced.
    fn checks(manifest: &Value, clusters: &[Value]) -> Vec<String> {
        let mut failures: Vec<Failure> = Vec::new();
        check_curated_precision(manifest, &json!({ "clusters": clusters }), &mut failures);
        failures.into_iter().map(|failure| failure.check).collect()
    }

    #[test]
    fn a_curated_non_duplicate_left_unclustered_passes() {
        assert!(
            checks(&manifest(&PAIR), &[cluster(&["a.py", "b.py"], false)]).is_empty(),
            "the report clusters other files and leaves the curated pair \
             alone, which is exactly what the entry asserts"
        );
        assert!(
            checks(&manifest(&PAIR), &[]).is_empty(),
            "a report with no clusters breaches no precision entry"
        );
    }

    #[test]
    fn a_curated_non_duplicate_shown_as_one_cluster_is_a_false_positive() {
        assert_eq!(
            checks(&manifest(&PAIR), &[cluster(&PAIR, false)]),
            vec!["precision".to_owned()],
            "a shown cluster spanning both curated paths is the false \
             positive the entry exists to name — seven open issues say \
             exactly this and none of them could be pinned before"
        );
    }

    #[test]
    fn a_hidden_cluster_does_not_breach_the_entry() {
        assert!(
            checks(&manifest(&PAIR), &[cluster(&PAIR, true)]).is_empty(),
            "precision is what the report shows. A cluster the user is never \
             shown makes no claim, so suppressing it is the fix, not a \
             loophole — the same visibility rule [CORPUS-RECALL] applies, \
             read in the opposite direction"
        );
    }

    #[test]
    fn an_entry_naming_fewer_than_two_files_fails_rather_than_passing() {
        assert_eq!(
            checks(&manifest(&[PAIR[0]]), &[cluster(&PAIR, false)]),
            vec!["precision".to_owned()],
            "one path cannot describe a pair the engine wrongly joined. It \
             must fail as uncurated rather than pass by spanning nothing"
        );
        assert_eq!(
            checks(&manifest(&[]), &[]),
            vec!["precision".to_owned()],
            "and an entry naming no files at all is the same defect"
        );
    }
}
