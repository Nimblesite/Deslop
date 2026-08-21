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

use super::declares_forbidden_supertype;
use crate::enclosure::Span;

/// Judges `source` in full, as `language`, against `supertype`.
fn declares(language: &str, source: &str, supertype: &str) -> bool {
    let span = Span::new("lib/widget.dart", 0, u64::try_from(source.len()).unwrap_or(0));
    declares_forbidden_supertype(language, source, &span, supertype)
        .expect("the predicate must judge a registered language")
}

/// Judges the sub-range of `source` delimited by `marker` .. end of source.
fn declares_from(language: &str, source: &str, marker: &str, supertype: &str) -> bool {
    let start = source.find(marker).expect("marker must exist in the fixture");
    let span = Span::new(
        "lib/widget.dart",
        u64::try_from(start).unwrap_or(0),
        u64::try_from(source.len()).unwrap_or(0),
    );
    declares_forbidden_supertype(language, source, &span, supertype)
        .expect("the predicate must judge a registered language")
}

/// A Dart widget that mentions both forbidden supertypes everywhere except
/// where it would matter: a doc comment, a line comment, a string literal,
/// and a constructor call in the body. It extends `State`, not either of
/// them.
const MENTIONS_ONLY: &str = r#"
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
"#;

#[test]
fn a_supertype_only_mentioned_in_comments_and_literals_is_not_a_declaration() {
    assert!(
        !declares("dart", MENTIONS_ONLY, "StatefulWidget"),
        "a doc comment, a line comment and a string literal all say \
         `extends StatefulWidget`; none of them declares it, and reporting \
         this cluster as framework boilerplate would suppress a real \
         finding on the strength of prose"
    );
    assert!(
        !declares("dart", MENTIONS_ONLY, "StatelessWidget"),
        "same in the other direction: `debugPrint('extends StatelessWidget')` \
         is an argument, not a base type"
    );
    assert!(
        declares("dart", MENTIONS_ONLY, "State"),
        "the class really does extend `State`, so the predicate must be able \
         to see the one declaration that is actually there — otherwise this \
         fixture would pass by seeing nothing at all"
    );
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
fn a_wrapped_extends_clause_is_the_same_declaration() {
    assert!(
        declares("dart", WRAPPED_DECLARATION, "StatefulWidget"),
        "`extends` and `StatefulWidget` are on different lines, so no \
         substring of the source spells `extends StatefulWidget` — the \
         declaration is identical and the gate must still see it"
    );
    assert!(
        !declares("dart", WRAPPED_DECLARATION, "StatelessWidget"),
        "the wrapped clause names one supertype; the other must not be \
         inferred from it"
    );
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
fn the_flat_declaration_still_fires_and_only_for_its_own_supertype() {
    assert!(
        declares("dart", FLAT_DECLARATION, "StatelessWidget"),
        "the ordinary spelling is what gh #331 is about; it must keep firing"
    );
    assert!(
        !declares("dart", FLAT_DECLARATION, "StatefulWidget"),
        "a StatelessWidget is not a StatefulWidget — a predicate that fired \
         on both would make the rule unfalsifiable"
    );
}

#[test]
fn a_member_of_the_declaration_is_judged_by_its_enclosing_class() {
    assert!(
        declares_from("dart", FLAT_DECLARATION, "  @override", "StatelessWidget"),
        "the ranked occurrence is usually the mandated member — Flutter's \
         `build`, or `createState` — not the class header. Judging only the \
         reported bytes would miss every cluster the rule exists to catch"
    );
}

#[test]
fn an_unregistered_language_fails_the_gate_rather_than_passing_it() {
    let span = Span::new("Ledger.fs", 0, 0);
    let verdict = declares_forbidden_supertype("fsharp", "", &span, "StatefulWidget");
    assert!(
        verdict.is_err(),
        "a manifest naming a language this module has no heritage grammar \
         for must fail loudly. Returning `false` would switch the precision \
         gate off silently, which is the [CORPUS-BASELINE] failure the \
         ratchet then reads as evidence the defect is absent"
    );
}
