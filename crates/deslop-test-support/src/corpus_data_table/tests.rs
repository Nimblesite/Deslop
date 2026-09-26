//! [CORPUS-PRECISION] Both directions of the data-table predicate, and what
//! the check says when it fires.
//!
//! The rule this replaced was `data_character_ratio(raw_source) >= 0.6` —
//! digits and literal separators over non-whitespace characters. It was
//! wrong in both directions at once, and these cases state each as an
//! assertion so neither can come back (gh #452):
//!
//! * a table of **string** literals carries no digits at all, so the rule
//!   scored it 0.00 and the gate let it rank at full logic weight — the
//!   precise outcome the check exists to forbid. `AGENTS.md` mandates
//!   named constants over literals, so well-formed code in a scanned
//!   corpus is exactly the code that rule could not see;
//! * ordinary logic carrying a digit-heavy comment — a version matrix, a
//!   range table, an RFC reference — scored 0.74 and the gate failed a
//!   report whose ranking was correct.
//!
//! A gate is also only worth what its message is worth: a reader who is told
//! a finding is empty, or sent to the wrong rank, cannot act on it. The
//! message cases pin the three things `data_table_rank` got wrong (gh #540) —
//! all three already correct in the sibling `boilerplate_rank` check.

use anyhow::{anyhow, Context, Result};
use deslop_core::wire_generated::{ClusterKind, ReportCluster, ReportOccurrence};
use serde_json::Value;

use super::{data_table_failure, occurrence_is_a_literal_table, MIN_TABLE_ENTRIES};
use crate::corpus::Failure;

/// The language the single-language fixtures are written in.
const PYTHON: &str = "python";
/// The language of the bundle the ranked-head message cases are drawn from.
const JAVASCRIPT: &str = "javascript";
/// The language of the two real occurrences that once failed the gate.
const TYPESCRIPT: &str = "typescript";

/// A font-metrics table as a bundler writes it: keys, arrays of numbers,
/// and nothing else.
const NUMERIC_TABLE: &str = "const METRICS = {989: [0.08167, 0.58167, 0, 0, 0.77778], \
                             1008: [0, 0.43056, 0.04028, 0, 0.66667], \
                             8245: [0, 0.54986, 0, 0, 0.275]};\n";
/// Ordinary duplicated logic, which must never be called a data table.
const REAL_LOGIC: &str = "function summarise(rows) { const total = rows.map((row) => \
                          row.amount).reduce(add, 0); return new Summary(total); }\n";

/// A table of string literals. No digits anywhere, so the character rule
/// scored it 0.00 and stayed silent.
const STRING_LITERAL_TABLE: &str = r#"STATE_ALABAMA = "Alabama"
STATE_ALASKA = "Alaska"
STATE_ARIZONA = "Arizona"
STATE_ARKANSAS = "Arkansas"
"#;

/// A numeric table — the shape the check was built for.
const NUMERIC_LIST: &str = "LOOKUP = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]\n";

/// Two literals side by side: a coincidence of ordinary logic, not a table.
const TWO_LITERALS: &str = "BOUNDS = [0, 1]\n";

/// Real logic whose comments are dense with version numbers and ranges.
/// The character rule scored this 0.74 and reported it as a table.
const LOGIC_WITH_DIGIT_HEAVY_COMMENTS: &str =
    "# Supported releases: 1.0.1, 1.0.2, 1.1.0, 1.2.0, 1.3.4, 2.0.0, 2.1.3,
# 2.2.0, 3.0.0, 3.1.1, 3.2.2, 4.0.0, 4.1.0, 5.0.0, 5.1.2, 6.0.0, 6.1.1.
# Ranges: 1-9, 10-19, 20-29, 30-39, 40-49, 50-59, 60-69, 70-79, 80-89.
value = first[index - 1] + second[index - 2] * 3
";

/// Ordinary logic with no literal table in sight.
const ORDINARY_LOGIC: &str = "def total(rows):
    return sum(row.amount for row in rows if row.active)
";

/// A config object holds nothing but literal arrays and still is not a
/// table: it is an argument to a call. The first shape of this predicate
/// flagged nest's `eslint.config.mjs` at rank 2 of the real corpus run,
/// because it searched for *any* all-literal collection anywhere in the
/// span rather than asking what the occurrence is.
const CONFIG_OBJECT_CALL: &str = "export default tseslint.config({
  ignores: ['node_modules', 'dist', 'coverage'],
});
";

/// A test setup block, likewise: literal-rich, but it calls.
/// Flagged at rank 9 of the same run.
const TEST_SETUP_BLOCK: &str = "beforeEach(async () => {
  const moduleRef = await Test.createTestingModule({
    providers: ['alpha', 'beta', 'gamma'],
  }).compile();
});
";

/// How many copies the cluster under test reports. The number a reader is
/// told must be this one.
const OCCURRENCES: usize = 34;
/// The zero-based index of the last cluster the check looks at, whose rank in
/// the report — and in the message — is one greater.
const LAST_HEAD_POSITION: usize = 9;
const LAST_HEAD_RANK: usize = 10;

const FIXTURE_PATH: &str = "internal/warpc/js/renderkatex.bundle.js";
const CHECK_NAME: &str = "data_table_rank";
const CLUSTER_ID: &str = "f88c8af320ac7e79";
const SEVERITY: &str = "warning";
const MASS: u64 = 1584;
const CANONICAL_NODES: usize = 56;

/// Every corpus language, with a literal table and a piece of logic in it.
///
/// A curated grammar naming the wrong node kinds is silently blind — the
/// predicate answers "not a table" for that whole language and the gate
/// stops asserting anything there. One row per language, both directions,
/// so a wrong kind fails by name.
const PER_LANGUAGE: &[(&str, &str, &str)] = &[
    (
        "python",
        "A = \"x\"\nB = \"y\"\nC = \"z\"\n",
        "def run(rows):\n    return sum(r.n for r in rows)\n",
    ),
    (
        "rust",
        "const A: i32 = 1;\nconst B: i32 = 2;\nconst C: i32 = 3;\n",
        "fn run(rows: &[Row]) -> usize { rows.iter().map(|r| r.n).sum() }\n",
    ),
    (
        "typescript",
        "const A = \"x\";\nconst B = \"y\";\nconst C = \"z\";\n",
        "function run(rows: Row[]) { return rows.map((r) => r.n).reduce(add); }\n",
    ),
    (
        "javascript",
        "const A = \"x\";\nconst B = \"y\";\nconst C = \"z\";\n",
        "function run(rows) { return rows.map((r) => r.n).reduce(add); }\n",
    ),
    (
        "csharp",
        "class C { const string A = \"x\"; const string B = \"y\"; const string D = \"z\"; }",
        "class C { int Run(Row[] rows) { return rows.Select(r => r.N).Sum(); } }",
    ),
    (
        "go",
        "const A = \"x\"\nconst B = \"y\"\nconst C = \"z\"\n",
        "func run(rows []Row) int {\n\ttotal := 0\n\tfor _, r := range rows {\n\t\ttotal += r.N\n\t}\n\treturn total\n}\n",
    ),
    (
        "dart",
        "const A = \"x\";\nconst B = \"y\";\nconst C = \"z\";\n",
        "int run(List<Row> rows) => rows.map((r) => r.n).reduce(add);\n",
    ),
    (
        "php",
        "<?php\nconst A = \"x\";\nconst B = \"y\";\nconst C = \"z\";\n",
        "<?php\nfunction run($rows) { return array_sum(array_map(fn($r) => $r->n, $rows)); }\n",
    ),
    (
        "fsharp",
        "let a = \"x\"\nlet b = \"y\"\nlet c = \"z\"\n",
        "let run rows = rows |> List.map (fun r -> r.N) |> List.sum\n",
    ),
];

/// A rendered cluster carrying `OCCURRENCES` copies, as JSON, exactly as the
/// check receives it from a report.
///
/// Built from the wire model rather than hand-written JSON, so the field names
/// the check reads are the field names a real report emits — reading one that
/// is not there is the whole of gh #540.
fn cluster() -> Result<Value> {
    let occurrences: Vec<ReportOccurrence> = (0..OCCURRENCES)
        .map(|index| ReportOccurrence {
            path: FIXTURE_PATH.into(),
            start_byte: index,
            end_byte: index.saturating_add(1),
            start_line: 1,
            end_line: 1,
            hidden: false,
            in_diff: None,
        })
        .collect();
    let cluster = ReportCluster {
        id: CLUSTER_ID.to_owned(),
        rank: 1,
        severity: SEVERITY.to_owned(),
        kind: ClusterKind::Identical,
        mass: MASS,
        canonical_node_count: CANONICAL_NODES,
        occurrences_total: occurrences.len(),
        occurrence_count: occurrences.len(),
        occurrences_truncated: false,
        occurrences,
        intersects_diff: None,
        is_newly_introduced: None,
    };
    serde_json::to_value(cluster).context("a wire cluster serialises")
}

/// The failure the check raises for `text` at `position`, as an error when it
/// raises none. Taken with `?` rather than `expect`, so a fixture that stopped
/// tripping the check fails by name instead of through a denied panic.
fn failure_for(position: usize, text: &str) -> Result<Failure> {
    data_table_failure(JAVASCRIPT, position, &cluster()?, text)?
        .ok_or_else(|| anyhow!("a table of literals at the head of the report is a breach"))
}

/// Judges `source` as Python in full.
fn is_table(source: &str) -> Result<bool> {
    occurrence_is_a_literal_table(PYTHON, source)
}

#[test]
fn a_ranked_data_table_is_reported_with_the_number_of_copies_it_actually_has() -> Result<()> {
    let failure = failure_for(0, NUMERIC_TABLE)?;
    assert_eq!(
        failure.check, CHECK_NAME,
        "the failure must keep its check id"
    );
    assert!(
        failure
            .detail
            .contains(&format!("{OCCURRENCES} occurrences")),
        "the message must name the {OCCURRENCES} copies the cluster reports. A reader told \
         a finding has 0 occurrences concludes the report contains an empty cluster, which \
         is not what happened. Message: {}",
        failure.detail,
    );
    assert!(
        !failure.detail.contains("0 occurrences"),
        "reading a field the report does not emit is what produced `0 occurrences` for \
         every breach. Message: {}",
        failure.detail,
    );
    Ok(())
}

#[test]
fn the_rank_in_the_message_is_the_rank_in_the_report() -> Result<()> {
    let failure = failure_for(LAST_HEAD_POSITION, NUMERIC_TABLE)?;
    assert!(
        failure.detail.contains(&format!("rank {LAST_HEAD_RANK}")),
        "ranks are one-based in the report, so the cluster at index {LAST_HEAD_POSITION} is \
         rank {LAST_HEAD_RANK}. A message naming rank {LAST_HEAD_POSITION} sends the reader \
         to the wrong finding. Message: {}",
        failure.detail,
    );
    Ok(())
}

#[test]
fn the_message_never_claims_a_category_the_report_does_not_carry() -> Result<()> {
    let failure = failure_for(0, NUMERIC_TABLE)?;
    for absent in ["categorised", "absent"] {
        assert!(
            !failure.detail.contains(absent),
            "no cluster carries a category — [RANK-CATEGORY] says a detection-time finding \
             kind is not carried as cluster metadata — so a message that reports one is \
             telling the reader about a field that does not exist, and the exemption it \
             implies can never open. Message: {}",
            failure.detail,
        );
    }
    Ok(())
}

#[test]
fn ordinary_duplicated_logic_is_not_called_a_data_table() -> Result<()> {
    assert!(
        !occurrence_is_a_literal_table(JAVASCRIPT, REAL_LOGIC)?,
        "a function that maps, reduces and constructs is logic; the calls it makes settle it"
    );
    assert!(
        data_table_failure(JAVASCRIPT, 0, &cluster()?, REAL_LOGIC)?.is_none(),
        "a cluster of real duplicated logic is exactly what the report is for; failing it \
         here would make the gate demand that genuine clones be demoted",
    );
    Ok(())
}

/// A table of string literals is a data table. The character rule could not
/// see it, because a table of names and strings holds no digits.
#[test]
fn a_table_of_string_literals_is_a_data_table() -> Result<()> {
    assert!(
        is_table(STRING_LITERAL_TABLE)?,
        "a run of `NAME = \"literal\"` declarations is a data table — there is no shared \
         control flow and nothing a reader could extract. Counting digits cannot see it, so \
         it ranked at full logic weight, which is what this check exists to forbid"
    );
    Ok(())
}

/// Logic is logic however many numbers its comments carry. Comments and
/// string contents are not the code's shape.
#[test]
fn logic_with_digit_heavy_comments_is_not_a_data_table() -> Result<()> {
    assert!(
        !is_table(LOGIC_WITH_DIGIT_HEAVY_COMMENTS)?,
        "an arithmetic expression under a version-matrix comment is logic; reporting it as a \
         data table fails a report whose ranking was correct"
    );
    Ok(())
}

/// The shape the check was built for still reads as a table.
#[test]
fn a_numeric_array_literal_is_a_data_table() -> Result<()> {
    assert!(
        is_table(NUMERIC_LIST)?,
        "a collection whose every element is a numeric literal is the canonical data table"
    );
    Ok(())
}

/// And ordinary logic still does not.
#[test]
fn ordinary_logic_is_not_a_data_table() -> Result<()> {
    assert!(
        !is_table(ORDINARY_LOGIC)?,
        "a function body with a comprehension is logic, not a table of literals"
    );
    Ok(())
}

/// Two literals side by side are a coincidence of ordinary logic; the floor
/// sits above them so the predicate errs towards calling a span logic.
#[test]
fn fewer_than_the_entry_floor_is_not_a_table() -> Result<()> {
    assert!(
        !is_table(TWO_LITERALS)?,
        "`[0, 1]` holds fewer than {MIN_TABLE_ENTRIES} entries, so it is not a table"
    );
    Ok(())
}

/// Every curated grammar sees its own language's constant table.
#[test]
fn every_corpus_language_recognises_its_own_literal_table() -> Result<()> {
    for (language, table, _) in PER_LANGUAGE {
        assert!(
            occurrence_is_a_literal_table(language, table)?,
            "the curated `{language}` table grammar does not recognise a three-entry constant \
             table, so the data-table gate is blind for every `{language}` repository"
        );
    }
    Ok(())
}

/// And none of them mistakes that language's ordinary logic for one.
#[test]
fn no_corpus_language_mistakes_logic_for_a_literal_table() -> Result<()> {
    for (language, _, logic) in PER_LANGUAGE {
        assert!(
            !occurrence_is_a_literal_table(language, logic)?,
            "the curated `{language}` table grammar reports ordinary logic as a data table, \
             which fails a report whose ranking was correct"
        );
    }
    Ok(())
}

/// A language with no curated grammar must fail loudly rather than answer
/// "not a table" for every occurrence it is handed.
#[test]
fn an_uncurated_language_errors_rather_than_judging_nothing() {
    let verdict = occurrence_is_a_literal_table("cobol", "MOVE 1 TO X.");
    assert!(
        verdict.is_err(),
        "an uncurated language must not answer a verdict for every occurrence it is handed — \
         that is how a gate quietly stops asserting anything"
    );
    let rendered = verdict
        .err()
        .map_or_else(String::new, |error| format!("{error}"));
    assert!(
        rendered.contains("cobol") && rendered.contains("curate"),
        "the error must name the language and ask for curation; got `{rendered}`"
    );
}

/// [CORPUS-PRECISION] A span that merely *contains* a literal array is
/// code holding data, not a data table. Both fixtures are the real
/// occurrences that failed `corpus_nest_typescript` before the call
/// exclusion landed (gh #452).
#[test]
fn code_that_merely_contains_a_literal_array_is_not_a_data_table() -> Result<()> {
    for (label, source) in [
        ("a config object passed to a call", CONFIG_OBJECT_CALL),
        ("a test setup block", TEST_SETUP_BLOCK),
    ] {
        assert!(
            !occurrence_is_a_literal_table(TYPESCRIPT, source)?,
            "{label} is code that holds data, not a table of literals; reporting it fails a \
             report whose ranking was correct"
        );
    }
    Ok(())
}
