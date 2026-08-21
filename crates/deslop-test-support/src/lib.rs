//! Cross-crate E2E test helpers shared by the `deslop`, `deslop-lsp`, and
//! `deslop-mcp` integration suites.
//!
//! Each binary crate previously carried its own byte-identical copies of the
//! version-manifest assertion, the LSP `Content-Length` JSON-RPC framing, and
//! the `deslop-lsp` child-spawn idiom. They live here once so the three test
//! suites share a single canonical implementation.
//!
//! Scope note: the LSP wire framing mirrors the shape that the upstream
//! `lspkit-server` toolkit owns on the production side. These are test-only
//! utilities (dev-dependency, never published); they do not duplicate the
//! production protocol shell, which stays in `deslop-lsp` / `deslop-mcp`.

pub mod corpus;
pub mod corpus_confidence;
pub mod corpus_precision;
pub mod corpus_scope;
pub mod enclosure;

use std::{
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use anyhow::{anyhow, Result};
use serde_json::Value;

/// Appends `.<ext>` to the file name of `base`, returning the new path.
/// E.g. `with_ext("/tmp/report", "json")` is `/tmp/report.json`. Used by the
/// CLI tests to derive the rendered report path from an `--output` prefix.
#[must_use]
pub fn with_ext(base: &Path, ext: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    let mut name = path
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_default();
    name.push(".");
    name.push(ext);
    path.set_file_name(name);
    path
}

/// Asserts that a `--version --format json` manifest carries the expected
/// shape: `manifestVersion = 1`, the given `name` and `kind`, the supplied
/// `version` string, and the fixed `language = "rust"` / `product = "deslop"`
/// fields every Deslop binary emits.
///
/// # Panics
///
/// Panics (failing the test) when any manifest field is absent or does not
/// match the expected value.
pub fn assert_version_manifest(value: &Value, name: &str, kind: &str, version: &str) {
    assert_eq!(value.get("manifestVersion"), Some(&Value::from(1)));
    assert_eq!(value.get("name"), Some(&Value::from(name)));
    assert_eq!(value.get("version").and_then(Value::as_str), Some(version));
    assert_eq!(value.get("kind"), Some(&Value::from(kind)));
    assert_eq!(value.get("language"), Some(&Value::from("rust")));
    assert_eq!(value.get("product"), Some(&Value::from("deslop")));
}

/// Writes one LSP-framed (`Content-Length`) payload to a child's stdin.
///
/// # Errors
///
/// Returns an error if writing the header, the body, or the flush fails.
pub fn write_lsp_frame(stdin: &mut ChildStdin, payload: &str) -> Result<()> {
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    stdin.write_all(header.as_bytes())?;
    stdin.write_all(payload.as_bytes())?;
    stdin.flush()?;
    Ok(())
}

/// Reads one LSP-framed JSON-RPC response: parses the `Content-Length` header
/// block, reads exactly that many body bytes, and deserialises them as JSON.
///
/// # Errors
///
/// Returns an error if the stream ends early, the `Content-Length` header is
/// missing or unparsable, or the body is not valid JSON.
pub fn read_lsp_frame(reader: &mut BufReader<ChildStdout>) -> Result<Value> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let _read = reader.read_line(&mut line)?;
        if line == "\r\n" {
            break;
        }
        if let Some(rest) = line.strip_prefix("Content-Length: ") {
            content_length = Some(rest.trim().parse::<usize>()?);
        }
    }
    let length = content_length.ok_or_else(|| anyhow!("missing Content-Length"))?;
    let mut buf = vec![0_u8; length];
    reader.read_exact(&mut buf)?;
    Ok(serde_json::from_slice(&buf)?)
}

/// Spawns the `deslop-lsp` binary against `root` with piped stdin/stdout and
/// the caller-chosen `stderr` disposition (`Stdio::piped()` to capture,
/// `Stdio::null()` to discard).
///
/// # Errors
///
/// Returns an error if the child process fails to spawn.
pub fn spawn_deslop_lsp(root: &Path, stderr: Stdio) -> Result<Child> {
    let bin = assert_cmd::cargo::cargo_bin("deslop-lsp");
    Ok(Command::new(bin)
        .arg(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(stderr)
        .spawn()?)
}

/// The hand-written copy-pasted logic clone used by the shared Dart
/// fixture below: a six-statement method body, duplicated verbatim, large
/// enough to cluster at the suites' `--min-nodes` floors.
const DUPLICATED_DART_METHOD: &str = "  int score(int v) {\n    final w = v * 3;\n    \
     final z = w + v;\n    final q = z - w;\n    final r = q * z;\n    \
     return r + w - v;\n  }\n";

/// Writes the shared "data table + copy-pasted logic" Dart fixture
/// ([RANK-CATEGORY] / [CLONE-NOISE-DART-DATA-TABLE-LITERAL]): a top-level
/// `List<HighlightData>` of near-identical constructor rows that the engine
/// labels `category="data"`, plus a verbatim-duplicated method across
/// `scorer_a.dart` / `scorer_b.dart` that stays `category="logic"`. Used by
/// the #190 demotion suite and the facet-surface suites ([FACET-TESTING])
/// so both categories exist in one corpus.
///
/// # Errors
///
/// Returns an error when the fixture directory or files cannot be written.
pub fn write_dart_data_table_fixture(src: &Path) -> Result<()> {
    use std::fmt::Write as _;
    std::fs::create_dir_all(src)?;
    let mut table = String::from(
        "class HighlightData {\n  const HighlightData({required this.title, \
         required this.wonder, required this.artifactId, required this.culture, \
         required this.date});\n  final String title;\n  final String wonder;\n  \
         final String artifactId;\n  final String culture;\n  final String date;\n}\n\n\
         const List<HighlightData> highlights = [\n",
    );
    let rows: &[(&str, &str, &str, &str)] = &[
        ("Pyramid", "Giza", "Egyptian", "2560 BC"),
        ("Great Wall", "China", "Chinese", "700 BC"),
        ("Petra", "Jordan", "Nabataean", "312 BC"),
        ("Colosseum", "Rome", "Roman", "80 AD"),
        ("Machu Picchu", "Peru", "Inca", "1450 AD"),
        ("Taj Mahal", "India", "Mughal", "1653 AD"),
        ("Chichen Itza", "Mexico", "Mayan", "600 AD"),
        ("Christ Redeemer", "Brazil", "Brazilian", "1931 AD"),
    ];
    for (index, (title, wonder, culture, date)) in rows.iter().enumerate() {
        let _ = writeln!(
            table,
            "  HighlightData(title: \"{title}\", wonder: \"{wonder}\", \
             artifactId: \"a{index}\", culture: \"{culture}\", date: \"{date}\"),"
        );
    }
    table.push_str("];\n");
    std::fs::write(src.join("highlight_data.dart"), table)?;
    std::fs::write(
        src.join("scorer_a.dart"),
        format!("class ScorerA {{\n{DUPLICATED_DART_METHOD}}}\n"),
    )?;
    std::fs::write(
        src.join("scorer_b.dart"),
        format!("class ScorerB {{\n{DUPLICATED_DART_METHOD}}}\n"),
    )?;
    Ok(())
}
