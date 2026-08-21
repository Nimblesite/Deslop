//! E2E regression for GH #69 [CLONE-NOISE-POLYMORPHIC-SIGNATURE].
//!
//! Four hosts implement one abstract `tool_call` contract. The kwargs-only
//! signature is forced to be identical in every implementation, but each
//! body drives a different backend (containers, machines, a mock, the
//! abstract stub). Nothing here can share a refactor, so no visible
//! cluster may ever pair two of these files — at any scope. A whole-file
//! view is the same polymorphic pattern seen wider, not a new finding:
//! promoting it reported both files 100% duplicated on the strength of
//! bytes the contract forces to agree.

mod common;

use crate::common::*;

#[test]
fn different_backend_implementations_never_pair_across_files() -> Result<()> {
    let scan_root = fixture("python-issue-69-abstract-method");
    let report = run_report(&scan_root, 4)?;
    let visible = visible_cluster_lines(&report);
    for cluster in clusters(&report) {
        let files = occurrence_files(cluster);
        let Some(first) = files.first() else {
            continue;
        };
        assert!(
            files.iter().all(|file| file == first),
            "every implementation of `tool_call` drives a different \
             backend, so a visible cluster pairing two of these files \
             reports the mandatory interface contract as duplication — \
             the whole-file view is the polymorphic pattern the filter \
             already suppresses at method scope, seen wider. Visible: \
             {visible:#?}"
        );
    }
    let docker_and_fly = clusters(&report).iter().any(|cluster| {
        let files = occurrence_files(cluster);
        files.iter().any(|file| file.contains("docker_host"))
            && files.iter().any(|file| file.contains("fly_host"))
    });
    assert!(
        !docker_and_fly,
        "docker indexes containers and builds a request body; fly indexes \
         machines and converts a response — a docker/fly pairing at any \
         scope is a false positive: {visible:#?}"
    );
    Ok(())
}
