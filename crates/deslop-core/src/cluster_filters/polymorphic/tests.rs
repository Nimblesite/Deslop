//! Unit tests for the polymorphic-signature filter's widened subject
//! resolution ([CLONE-NOISE-POLYMORPHIC-SIGNATURE]).
//!
//! The spec's widened direction is language-agnostic: when a member
//! view is wider than any single function, the subject is "the sole
//! function the range contains with nothing but declaration scaffolding
//! (imports, docstrings, the class shell) around it". The E2E red
//! (`issue_331_template_stamped_widget_scaffolds_do_not_surface`) pins
//! the user-visible outcome; the tests here pin the seam itself — a
//! whole-class Dart view must resolve its sole `@override` method as
//! the subject, which is what lets the framework-scaffold conviction
//! fire at all.

use std::collections::HashMap;

use super::super::{collect_snippets, is_noise_pattern, NoiseFilter, ParseCache};
use crate::{
    ast::ByteRange,
    fingerprint::Fingerprint,
    state::{FileId, FileRegistry},
};

/// The template-stamped Flutter widget scaffold, parameterised by the
/// one body expression a template stamps per app — the #331 corpus.
fn widget_scaffold(body: &str) -> String {
    scaffold(body, FRAMEWORK_BASE, OVERRIDE_MARKER)
}

/// Known Flutter base class.
const FRAMEWORK_BASE: &str = "StatelessWidget";
/// Ordinary base class without a known framework contract.
const GENERIC_BASE: &str = "DomainBase";
/// Explicit override proof.
const OVERRIDE_MARKER: &str = "@override";
/// Class member without an override marker.
const NO_MARKER: &str = "";
/// Language of every fixture in this module.
const DART: &str = "dart";

/// The same scaffold with the `@override` annotation removed — an
/// ordinary same-named class, no compiler proof of any contract.
fn widget_scaffold_without_marker(body: &str) -> String {
    scaffold(body, FRAMEWORK_BASE, NO_MARKER)
}

/// Builds one class with the requested contract and body.
fn scaffold(body: &str, base: &str, marker: &str) -> String {
    format!(
        "class ExampleApp extends {base} {{\n\
         \x20 const ExampleApp({{super.key}});\n\
         \x20 {marker}\n\
         \x20 Widget build(BuildContext context) {{\n\
         \x20   return MaterialApp(home: {body});\n\
         \x20 }}\n\
         }}\n"
    )
}

/// The four bodies the #331 fixture stamps into the scaffold. Each
/// diverges in normalised shape, so the bodies-differ requirement holds.
const STAMPED_BODIES: [&str; 4] = [
    "Text(\"alpha\")",
    "Column(children: [Text(\"beta\")])",
    "Container(width: 4, color: Colors.red)",
    "ListView(shrinkWrap: true)",
];

/// One whole-file fingerprint per source, in registration order — the
/// member shape a module-wide fused view produces.
struct Component {
    members: Vec<Fingerprint>,
    sources: HashMap<FileId, Vec<u8>>,
    languages: HashMap<FileId, &'static str>,
}

impl Component {
    /// Registers one Dart file per `bodies` entry, fingerprinted over
    /// its whole extent.
    fn across_files(bodies: &[String]) -> Self {
        let mut registry = FileRegistry::new();
        let mut component = Self {
            members: Vec::new(),
            sources: HashMap::new(),
            languages: HashMap::new(),
        };
        for body in bodies {
            let file_id =
                registry.register(format!("example_{}.dart", component.members.len()).into());
            let source = body.clone().into_bytes();
            let end = source.len();
            let _previous = component.sources.insert(file_id, source);
            let _language = component.languages.insert(file_id, DART);
            component.members.push(Fingerprint {
                hash: [0_u8; 32],
                file_id,
                byte_range: ByteRange { start: 0, end },
                node_count: 20,
            });
        }
        component
    }

    /// Isolates the polymorphic rule from independent language rules.
    fn verdict(&self, cache: &ParseCache) -> Option<NoiseFilter> {
        let snippets = collect_snippets(&self.members, &self.sources, DART, cache)?;
        super::is_polymorphic_signature_cluster(&snippets, &self.sources, &self.languages, cache)
            .then_some(NoiseFilter::Polymorphic)
    }

    /// The combined verdict, including independent language-specific rules.
    fn all_noise_verdict(&self, cache: &ParseCache) -> Option<NoiseFilter> {
        is_noise_pattern(&self.members, &self.sources, &self.languages, cache)
    }
}

/// [CLONE-NOISE-POLYMORPHIC-SIGNATURE] / #331: template-stamped widget
/// scaffolds are framework-mandated mirrors — every member resolves to
/// the sole `@override build` behind a class shell, the name is shared,
/// the bodies differ in shape, and the marker proves the contract the
/// scan never reaches. The polymorphic filter must convict the
/// component so it never surfaces as duplication.
#[test]
fn framework_stamped_dart_widget_scaffolds_are_convicted_by_the_override_marker() {
    let bodies: Vec<String> = STAMPED_BODIES
        .iter()
        .map(|body| widget_scaffold(body))
        .collect();
    let component = Component::across_files(&bodies);
    assert_eq!(
        component.verdict(&ParseCache::new()),
        Some(NoiseFilter::Polymorphic),
        "a whole-class Dart view must resolve its sole @override build as \
         the polymorphic subject, so the framework-mandated scaffold is \
         convicted instead of surfacing as duplication"
    );
}

/// The marker is the proof. The same classes without `@override`
/// implement nothing the index cannot see, so removing the marker must
/// leave the component unconvicted by the polymorphic rule. The independent
/// Flutter rule is tested separately below.
#[test]
fn the_same_scaffolds_without_override_markers_are_not_convicted() {
    let bodies: Vec<String> = STAMPED_BODIES
        .iter()
        .map(|body| widget_scaffold_without_marker(body))
        .collect();
    let component = Component::across_files(&bodies);
    assert_eq!(
        component.verdict(&ParseCache::new()),
        None,
        "without an override marker the polymorphic rule has no contract proof"
    );
}

/// [CLONE-NOISE-DART-WIDGET-SCAFFOLD] Flutter scaffolds have a separate noise rule.
#[test]
fn known_widget_scaffolding_uses_its_language_rule_without_an_override_marker() {
    let bodies: Vec<String> = STAMPED_BODIES
        .iter()
        .map(|body| widget_scaffold_without_marker(body))
        .collect();
    let component = Component::across_files(&bodies);
    assert_eq!(component.verdict(&ParseCache::new()), None);
    assert_eq!(
        component.all_noise_verdict(&ParseCache::new()),
        Some(NoiseFilter::LanguageSpecific)
    );
}

/// [CLONE-NOISE-POLYMORPHIC-SIGNATURE] Unknown bases do not prove a contract.
#[test]
fn ordinary_subclasses_without_contract_proof_are_not_convicted() {
    let bodies: Vec<String> = STAMPED_BODIES
        .iter()
        .map(|body| scaffold(body, GENERIC_BASE, NO_MARKER))
        .collect();
    let component = Component::across_files(&bodies);
    assert_eq!(component.verdict(&ParseCache::new()), None);
    assert_eq!(component.all_noise_verdict(&ParseCache::new()), None);
}

/// A copy-pasted override is never suppressed: the conviction requires
/// bodies that differ in normalised shape, so byte-identical overrides
/// must surface no matter what the marker says.
#[test]
fn byte_identical_overrides_are_not_convicted() {
    let bodies: Vec<String> = ["Text(\"alpha\")", "Text(\"alpha\")"]
        .iter()
        .map(|body| widget_scaffold(body))
        .collect();
    let component = Component::across_files(&bodies);
    assert_eq!(
        component.verdict(&ParseCache::new()),
        None,
        "the filter requires differing bodies; a byte-identical override \
         pair is genuine copy-paste and must keep surfacing"
    );
}
