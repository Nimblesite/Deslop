//! [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] A live session that has absorbed any sequence of edits reports exactly what a cold session over the same tree reports.
//!
//! Candidate pairs are drawn as stars: each structural bucket and each LSH band group pairs its first member, in path order, with every other. An edit can therefore change pairs between files it never touched: a new file that sorts first becomes the centre, a rewritten centre hands the star to the next file, a deleted member shrinks it. Every step below moves a star that way, over one long-lived session, and is compared field for field with a cold session.

#![cfg(feature = "live")]

use std::{collections::BTreeSet, fs, path::Path};

use anyhow::{Context, Result};
use deslop_core::{live::AnalysisSession, report::Report};
use serde_json::Value;

use crate::common::live_session;

/// The report field that records cache reuse, the one permitted difference between a live and a cold report.
const CACHE_STATS_FIELD: &str = "cache_stats";
/// The loop every exact copy carries.
const LOOP_BODY: &str = "            int total = 0;\n            for (int index = 0; index < input; index = index + 1)\n            {\n                total = total + index;\n            }\n";
/// One statement that turns an exact copy into a near copy.
const NEAR_STATEMENT: &str = "            total = total + input;\n";
/// A second, unrelated statement, so two near copies differ from each other too.
const FAR_STATEMENT: &str = "            total = total * 2 - input;\n";

/// What a file holds at one step.
#[derive(Clone, Copy)]
enum Content {
    /// One exact copy of the loop method.
    Exact,
    /// The loop method plus one statement.
    Near,
    /// The loop method plus two statements.
    Nearer,
    /// Two exact copies in one file.
    Twice,
    /// Code that shares nothing with the loop method; every file holding it is a copy of the others.
    Unrelated,
    /// Raw text, for a configuration file or a file no parser reads.
    Text(&'static str),
    /// The file is deleted.
    Gone,
}

/// One C# method around the loop, with `extra` statements after it.
fn method(name: &str, extra: &str) -> String {
    format!(
        "        public int {name}(int input)\n        {{\n            if (input < 0)\n            {{\n                return 0;\n            }}\n{LOOP_BODY}{extra}            return total;\n        }}\n"
    )
}

/// A class holding `methods`, in a namespace named after the file.
fn class_source(stem: &str, methods: &str) -> String {
    format!("namespace Space{stem}\n{{\n    public class Processor{stem}\n    {{\n{methods}    }}\n}}\n")
}

/// A method with no loop, no branch and no shared statement.
fn unrelated_source(stem: &str) -> String {
    let body = format!(
        "        public string Describe(string owner)\n        {{\n            var parts = new System.Collections.Generic.List<string>();\n            parts.Add(owner);\n            parts.Add(\"{stem}\");\n            parts.Sort();\n            return string.Join(\"|\", parts);\n        }}\n"
    );
    class_source(stem, &body)
}

/// The source text for `content` in the file `stem`, or `None` for a deletion.
fn source_for(stem: &str, content: Content) -> Option<String> {
    let near = format!("{NEAR_STATEMENT}{FAR_STATEMENT}");
    Some(match content {
        Content::Exact => class_source(stem, &method("Compute", "")),
        Content::Near => class_source(stem, &method("Compute", NEAR_STATEMENT)),
        Content::Nearer => class_source(stem, &method("Compute", &near)),
        Content::Twice => class_source(
            stem,
            &format!("{}{}", method("Compute", ""), method("Again", "")),
        ),
        Content::Unrelated => unrelated_source(stem),
        Content::Text(text) => text.to_owned(),
        Content::Gone => return None,
    })
}

/// Writes or deletes one file and returns its path.
fn land(root: &Path, name: &str, content: Content) -> Result<std::path::PathBuf> {
    let path = root.join(name);
    let stem = name.split('.').next().unwrap_or(name);
    match source_for(stem, content) {
        Some(source) => fs::write(&path, source).with_context(|| format!("write {name}"))?,
        None => fs::remove_file(&path).with_context(|| format!("delete {name}"))?,
    }
    Ok(path)
}

/// A report as comparable JSON, without the cache-reuse record.
fn comparable(report: &Report) -> Result<Value> {
    let mut value = serde_json::to_value(report).context("serialise report")?;
    if let Some(fields) = value.as_object_mut() {
        drop(fields.remove(CACHE_STATS_FIELD));
    }
    Ok(value)
}

/// The file stems that appear in any reported cluster.
fn clustered_stems(report: &Report) -> BTreeSet<String> {
    report
        .clusters
        .iter()
        .flat_map(|cluster| cluster.occurrences.iter())
        .filter_map(|occurrence| occurrence.path.file_stem())
        .map(|stem| stem.to_string_lossy().into_owned())
        .collect()
}

/// One pass: the files it changes, and the files that must sit in a cluster afterwards.
struct Step {
    /// What the step proves.
    name: &'static str,
    /// Every file the pass changes, with its new content.
    changes: &'static [(&'static str, Content)],
    /// The stems reported in at least one cluster once the pass lands.
    clustered: &'static [&'static str],
    /// Whether the pass changes an analysed file and so publishes a generation.
    publishes: bool,
}

/// The edit history. File names sort `a < m < n < p < z`, so `a` is every star's centre while it holds a copy.
const HISTORY: [Step; 9] = [
    Step {
        name: "a new first file becomes the centre of the star",
        changes: &[("a.cs", Content::Exact)],
        clustered: &["a", "m", "n", "p", "z"],
        publishes: true,
    },
    Step {
        name: "the centre is rewritten and hands the star on",
        changes: &[("a.cs", Content::Unrelated)],
        clustered: &["m", "n", "p", "z"],
        publishes: true,
    },
    Step {
        name: "the new centre is deleted",
        changes: &[("m.cs", Content::Gone)],
        clustered: &["n", "p", "z"],
        publishes: true,
    },
    Step {
        name: "a near copy drifts further from its neighbours",
        changes: &[("p.cs", Content::Nearer)],
        clustered: &["n", "p", "z"],
        publishes: true,
    },
    Step {
        name: "the first file takes the centre back with two copies",
        changes: &[("a.cs", Content::Twice)],
        clustered: &["a", "n", "p", "z"],
        publishes: true,
    },
    Step {
        name: "one pass moves the centre, a middle member and the last member",
        changes: &[
            ("a.cs", Content::Gone),
            ("n.cs", Content::Near),
            ("z.cs", Content::Twice),
        ],
        clustered: &["n", "p", "z"],
        publishes: true,
    },
    Step {
        name: "a deleted file returns as an exact copy",
        changes: &[("m.cs", Content::Exact)],
        clustered: &["m", "n", "p", "z"],
        publishes: true,
    },
    Step {
        name: "three files leave the loop family for a family of their own",
        changes: &[
            ("m.cs", Content::Unrelated),
            ("n.cs", Content::Unrelated),
            ("p.cs", Content::Unrelated),
        ],
        clustered: &["m", "n", "p", "z"],
        publishes: true,
    },
    Step {
        name: "the last loop copies join the other family and the loop family is gone",
        changes: &[("z.cs", Content::Unrelated)],
        clustered: &["m", "n", "p", "z"],
        publishes: true,
    },
];

/// The tree before the first edit: three exact or near copies and no first file yet.
const SEED: [(&str, Content); 4] = [
    ("m.cs", Content::Exact),
    ("n.cs", Content::Exact),
    ("p.cs", Content::Near),
    ("z.cs", Content::Exact),
];

/// Asserts the live report names the expected files and equals a cold session's report over the same tree.
fn assert_matches_cold(session: &AnalysisSession, root: &Path, step: &Step) -> Result<()> {
    let live = session.report();
    let expected: BTreeSet<String> = step
        .clustered
        .iter()
        .map(|stem| (*stem).to_owned())
        .collect();
    assert_eq!(
        clustered_stems(&live),
        expected,
        "{}: clustered files",
        step.name
    );
    let cold = live_session(root)?.report();
    assert_eq!(
        comparable(&live)?,
        comparable(&cold)?,
        "{}: live report differs from cold",
        step.name
    );
    Ok(())
}

/// The loop family's files once the seed has landed.
const SEEDED_FAMILY: [&str; 4] = ["m", "n", "p", "z"];

/// Lands one step's changes and feeds them through a single pass.
fn apply_step(session: &mut AnalysisSession, root: &Path, step: &Step) -> Result<()> {
    let changed = step
        .changes
        .iter()
        .map(|(name, content)| land(root, name, *content))
        .collect::<Result<Vec<_>>>()?;
    let before = session.generation();
    drop(session.apply_changes(&changed).with_context(|| step.name)?);
    let published = session.generation() > before;
    assert_eq!(
        published, step.publishes,
        "{}: publishes a generation",
        step.name
    );
    Ok(())
}

/// Seeds one session, walks `history` through it and compares every step with a cold session.
fn assert_history_matches_cold(history: &[Step]) -> Result<()> {
    let workspace = tempfile::tempdir().context("tempdir")?;
    let root = workspace.path();
    for (name, content) in SEED {
        drop(land(root, name, content)?);
    }
    let mut session = live_session(root)?;
    let seeded = Step {
        name: "seed",
        changes: &[],
        clustered: &SEEDED_FAMILY,
        publishes: false,
    };
    assert_matches_cold(&session, root, &seeded)?;
    for step in history {
        apply_step(&mut session, root, step)?;
        assert_matches_cold(&session, root, step)?;
    }
    Ok(())
}

#[test]
fn every_edit_in_a_long_session_matches_the_cold_report() -> Result<()> {
    assert_history_matches_cold(&HISTORY)
}

/// A configuration that scopes the second loop copy out of the corpus.
const EXCLUDE_N: &str = "[defaults]\nexclude = [\"n.cs\"]\n";
/// A configuration that scopes nothing out.
const EXCLUDE_NOTHING: &str = "[defaults]\nexclude = []\n";
const CONFIG_FILE: &str = ".deslop.toml";
/// A file no parser reads.
const LOG_FILE: &str = "build.log";

/// Edits on either side of a rescoped corpus: a pass that re-reads the configuration rebuilds every pair, and the edits after it must still match cold.
const RESCOPED_HISTORY: [Step; 6] = [
    Step {
        name: "the configuration scopes a member out",
        changes: &[(CONFIG_FILE, Content::Text(EXCLUDE_N))],
        clustered: &["m", "p", "z"],
        publishes: true,
    },
    Step {
        name: "an edit lands while the member is scoped out",
        changes: &[("m.cs", Content::Near)],
        clustered: &["m", "p", "z"],
        publishes: true,
    },
    Step {
        name: "the scoped-out file is edited and stays out",
        changes: &[("n.cs", Content::Twice)],
        clustered: &["m", "p", "z"],
        publishes: false,
    },
    Step {
        name: "the configuration scopes the member back in, as edited",
        changes: &[(CONFIG_FILE, Content::Text(EXCLUDE_NOTHING))],
        clustered: &["m", "n", "p", "z"],
        publishes: true,
    },
    Step {
        name: "a file no parser reads changes nothing",
        changes: &[(LOG_FILE, Content::Text("linking\n"))],
        clustered: &["m", "n", "p", "z"],
        publishes: false,
    },
    Step {
        name: "an edit after the rescope and the inert pass",
        changes: &[("z.cs", Content::Unrelated), ("a.cs", Content::Exact)],
        clustered: &["a", "m", "n", "p"],
        publishes: true,
    },
];

#[test]
fn edits_around_a_rescoped_corpus_match_the_cold_report() -> Result<()> {
    assert_history_matches_cold(&RESCOPED_HISTORY)
}
