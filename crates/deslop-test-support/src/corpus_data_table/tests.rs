//! [CORPUS-PRECISION] Both directions of the data-table predicate (gh #452).
//!
//! The shipped rule is `data_character_ratio(raw_source) >= 0.6` — digits
//! and literal separators over non-whitespace characters. It is wrong in
//! both directions at once, and these cases state each as an assertion so
//! neither can come back:
//!
//! * a table of **string** literals carries no digits at all, so the rule
//!   scores it 0.00 and the gate lets it rank at full logic weight — the
//!   precise outcome the check exists to forbid. `AGENTS.md` mandates
//!   named constants over literals, so well-formed code in a scanned
//!   corpus is exactly the code this rule cannot see;
//! * ordinary logic carrying a digit-heavy comment — a version matrix, a
//!   range table, an RFC reference — scores 0.74 and the gate fails a
//!   report whose `logic` categorisation was correct.
//!
//! The replacement reads the occurrence's AST, never characters of raw
//! text: a table is a run of declarations or collection elements whose
//! values are all literal leaves.

use anyhow::Result;

use super::occurrence_is_a_literal_table;
use crate::enclosure::Span;

/// The language every fixture here is written in.
const PYTHON: &str = "python";

/// The fixture path a span names; nothing reads it from disk.
const FIXTURE_PATH: &str = "tables.py";

/// A table of string literals. No digits anywhere, so the shipped
/// character rule scores it 0.00 and stays silent.
const STRING_LITERAL_TABLE: &str = r#"STATE_ALABAMA = "Alabama"
STATE_ALASKA = "Alaska"
STATE_ARIZONA = "Arizona"
STATE_ARKANSAS = "Arkansas"
"#;

/// A numeric table — the shape the check was built for.
const NUMERIC_TABLE: &str = "LOOKUP = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]\n";

/// Real logic whose comments are dense with version numbers and ranges.
/// The shipped character rule scores this 0.74 and reports it as a table.
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

/// Judges `source` in full.
///
/// # Errors
///
/// Propagates the predicate's own error — a span outside the source, an
/// unregistered language, or a parse failure.
fn is_table(source: &str) -> Result<bool> {
    let span = Span::new(FIXTURE_PATH, 0, u64::try_from(source.len()).unwrap_or(0));
    occurrence_is_a_literal_table(PYTHON, source, &span)
}

/// A table of string literals is a data table. The character rule cannot
/// see it, because a table of names and strings holds no digits.
#[test]
fn a_table_of_string_literals_is_a_data_table() -> Result<()> {
    assert!(
        is_table(STRING_LITERAL_TABLE)?,
        "a run of `NAME = \"literal\"` declarations is a data table — there is no shared \
         control flow and nothing a reader could extract. Counting digits cannot see it, so \
         it ranks at full logic weight, which is what this check exists to forbid"
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
         data table fails a report whose categorisation was correct"
    );
    Ok(())
}

/// The shape the check was built for still reads as a table.
#[test]
fn a_numeric_array_literal_is_a_data_table() -> Result<()> {
    assert!(
        is_table(NUMERIC_TABLE)?,
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

/// Every corpus language, with a literal table and a piece of logic in it.
///
/// A curated grammar naming the wrong node kinds is silently blind — the
/// predicate answers "not a table" for that whole language and the gate
/// stops asserting anything there, which is the gh #439 failure mode. One
/// row per language, both directions, so a wrong kind fails by name.
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

/// Judges `source` as `language`.
///
/// # Errors
///
/// Propagates an uncurated language, a span outside the source, or a
/// parse failure.
fn is_table_in(language: &str, source: &str) -> Result<bool> {
    let span = Span::new(FIXTURE_PATH, 0, u64::try_from(source.len()).unwrap_or(0));
    occurrence_is_a_literal_table(language, source, &span)
}

/// Every curated grammar sees its own language's constant table.
#[test]
fn every_corpus_language_recognises_its_own_literal_table() -> Result<()> {
    for (language, table, _) in PER_LANGUAGE {
        assert!(
            is_table_in(language, table)?,
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
            !is_table_in(language, logic)?,
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
    let verdict = is_table_in("cobol", "MOVE 1 TO X.");
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
            !is_table_in("typescript", source)?,
            "{label} is code that holds data, not a table of literals; reporting it fails a \
             report whose ranking was correct"
        );
    }
    Ok(())
}
