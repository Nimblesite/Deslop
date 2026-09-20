//! [CLONE-KIND-LABELS] [FUSED-PAIR-SIGNALS] What a diagnostic message
//! is allowed to say, over the shared fixtures in `super`.

use super::*;

#[test]
fn fixture_kind_labels_are_the_registry_labels() {
    let fixture = deslop_core::report_fixtures::FIXTURE_KIND;
    assert_eq!(fixture.wire_label(), FIXTURE_KIND_WIRE);
    assert_eq!(fixture.labels().title, FIXTURE_KIND_TITLE);
}

#[test]
fn diagnostic_message_shows_kind_count_and_mass() {
    let message = diagnostic_message(&two_file_cluster());
    assert!(message.contains(" — "), "joined with em dash: {message}");
    assert!(
        message.starts_with(&format!("{FIXTURE_KIND_TITLE} × 2")),
        "kind title and instance count first: {message}"
    );
    assert!(
        message.contains(&format!("mass {HEAVY_CLUSTER_MASS}")),
        "message carries the duplicated mass: {message}"
    );
    assert!(
        !message.contains("Type-"),
        "diagnostic message must not expose clone taxonomy labels: {message}"
    );
}

// [FUSED-PAIR-SIGNALS] The admission signals are pair measurements and
// never touch the cluster. An LSP diagnostic on one occurrence must not
// render them: the message quotes the folded kind, the count and the
// duplicated mass, and nothing else.
#[test]
fn diagnostic_message_renders_no_pair_evidence() {
    let cluster = two_file_cluster();
    let message = diagnostic_message(&cluster);
    assert!(
        message.contains(&format!("{FIXTURE_KIND_TITLE} × 2")),
        "the kind title and count survive: {message}"
    );
    assert!(
        !message.contains("fused"),
        "no cluster fused score on any surface: {message}"
    );
    for axis in [
        "structural",
        "jaccard",
        "embedding",
        "agreement",
        "rename",
        "literal",
    ] {
        assert!(
            !message.contains(axis),
            "pair evidence must not reach the diagnostic ({axis}): {message}"
        );
    }
    assert!(
        !message.contains("measured pair") && !message.contains("occurrences 1 and 2"),
        "no pair attribution on a cluster surface: {message}"
    );
}

// The message is a pure function of count and duplicated mass: two
// clusters with the same membership shape and mass quote the same text
// regardless of any pair measurements, and a mass difference shows.
#[test]
fn diagnostic_message_depends_on_count_and_mass_only() {
    let same_mass = sample_cluster(
        "twin",
        HEAVY_CLUSTER_MASS,
        vec![occurrence(A_FILE, 0, 1), occurrence("b.cs", 0, 1)],
    );
    assert_eq!(
        diagnostic_message(&same_mass),
        diagnostic_message(&two_file_cluster()),
        "same count and mass → same message: {}",
        diagnostic_message(&same_mass)
    );
    let heavier = sample_cluster(
        "heavy",
        HEAVY_CLUSTER_MASS * 2,
        vec![occurrence(A_FILE, 0, 1), occurrence("b.cs", 0, 1)],
    );
    assert_ne!(
        diagnostic_message(&heavier),
        diagnostic_message(&two_file_cluster()),
        "mass must show in the message: {} vs {}",
        diagnostic_message(&heavier),
        diagnostic_message(&two_file_cluster())
    );
}

#[test]
fn diagnostic_never_renders_pair_scores() {
    let cluster = sample_cluster(
        "unsourced",
        HEAVY_CLUSTER_MASS,
        vec![occurrence(A_FILE, 0, 1)],
    );
    let message = diagnostic_message(&cluster);
    assert!(message.contains(&format!("{FIXTURE_KIND_TITLE} × 1")));
    assert!(
        !message.contains("structural"),
        "unsourced structural score leaked: {message}"
    );
    assert!(
        !message.contains("agreement"),
        "unsourced content score leaked: {message}"
    );
}
