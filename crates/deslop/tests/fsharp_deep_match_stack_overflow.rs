//! [PIPELINE-NORMALIZE-AST] A pathologically deep F# file whose AST nests
//! *just under* [`MAX_AST_DEPTH`] must not abort the run.
//!
//! `MAX_AST_DEPTH` (500) is documented as the chokepoint that keeps deep
//! structure away from the pipeline's recursive walks, "well under the
//! overflow threshold on ... the CLI's 8 MB main thread". It is not. A file
//! the guard *accepts* still overflows the stack, and the process dies with
//! no report at all — every duplicate in every other file is lost.
//!
//! Found by scanning the pinned `fsharp` corpus (dotnet/fsharp v15.2.302),
//! which contains a real instance:
//! `tests/fsharp/core/large/matches/LargeMatches-maxtested.fs`. Scanning
//! that one 9 KB file exits `0xC00000FD` (`STATUS_STACK_OVERFLOW`) with
//! `thread 'main' has overflowed its stack`, so the whole 6233-file corpus
//! reports nothing.
//!
//! The failure is non-monotonic in depth, which is why #168's 5000-deep
//! Dart fixture never caught it — 5000 is far *above* the guard, so that
//! file is rejected and skipped safely. Measured on the nested-`match`
//! shape below, one nesting level at a time:
//!
//! | nested matches | outcome                          |
//! |----------------|----------------------------------|
//! | 140            | accepted, analysed, report written |
//! | 150 – 164      | accepted, **stack overflow, no report** |
//! | 165 and deeper | rejected by the guard, skipped, report written |
//!
//! So the dangerous band is precisely the depths the guard lets through.
//! This test pins the deepest accepted inputs.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde_json::Value;

mod common;
use crate::common::*;

/// Nesting depths the depth guard accepts and the recursive walks then
/// overflow on. 164 is the deepest input the guard admits; 165 is rejected.
const ACCEPTED_BUT_OVERFLOWING: [usize; 4] = [150, 156, 160, 164];

fn report_path(tmp: &Path) -> PathBuf {
    let mut path = tmp.join("report");
    let _replaced = path.set_extension("json");
    path
}

/// An F# function of `nesting` right-nested `match` expressions — the shape
/// `LargeMatches-maxtested.fs` uses, reduced to the smallest form that
/// reproduces.
fn nested_matches(nesting: usize) -> String {
    let head = "module Deep\n\
        let rnd = new System.Random()\n\
        let r() = if rnd.Next(3) > 1 then Some 4 else None\n\
        let f() =\n";
    let arm = "    match r() with\n    | Some x -> x\n    | None ->\n";
    format!("{head}{}    0\n", arm.repeat(nesting))
}

#[test]
fn deep_fsharp_matches_under_the_guard_do_not_abort_the_run() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let src = tmp.path().join("src");
    fs::create_dir(&src)?;

    for nesting in ACCEPTED_BUT_OVERFLOWING {
        fs::write(
            src.join(format!("deep_{nesting}.fs")),
            nested_matches(nesting),
        )?;
    }

    // Two byte-identical helpers: a genuine clone cluster proves the rest of
    // the corpus is still analysed once the deep files are handled.
    let helper = "module Helper\n\
        let combine (first: int) (second: int) =\n    \
            let total = first + second\n    \
            total * total\n";
    fs::write(src.join("alpha.fs"), helper)?;
    fs::write(src.join("beta.fs"), helper)?;

    let report = report_path(tmp.path());
    let mut cmd = deslop_cmd(&src, &tmp.path().join("report"))?;
    let _assertion = cmd
        .args([
            "--min-nodes",
            "5",
            "--embeddings",
            "off",
            "--notext",
            "--nohtml",
        ])
        .assert()
        .success();

    // A crashed scan writes no report at all, so reading it is itself an
    // assertion that the run survived every deep file.
    let body = fs::read_to_string(&report)?;
    let json: Value = serde_json::from_str(&body)?;

    let clusters = clusters(&json);
    assert!(
        !clusters.is_empty(),
        "the duplicated alpha/beta helper must still cluster after the deep \
         files are handled: {body}"
    );

    let helper_cluster = clusters
        .iter()
        .find(|cluster| {
            occurrences(cluster).iter().any(|occurrence| {
                occurrence_path(occurrence).is_ok_and(|path| path.ends_with("alpha.fs"))
            })
        })
        .ok_or_else(|| anyhow::anyhow!("no cluster covers alpha.fs: {body}"))?;

    let paths: Vec<&str> = occurrences(helper_cluster)
        .iter()
        .filter_map(|occurrence| occurrence_path(occurrence).ok())
        .collect();
    assert!(
        paths.iter().any(|path| path.ends_with("alpha.fs"))
            && paths.iter().any(|path| path.ends_with("beta.fs")),
        "the alpha/beta cluster must name both files, got {paths:?}: {body}"
    );
    Ok(())
}
