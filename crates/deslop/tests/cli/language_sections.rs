use super::support::*;

// Two Rust files that are renamed (Type-2) clones of one function: the
// copy renames the function and its parameter but keeps the body's
// locals, so most collapsed-leaf content still agrees and the pair
// routes to `nearly_identical` under [FUSION-CONTENT-GATE]. A fully
// renamed copy carries no content evidence and honestly routes to
// `structural_only` instead. Shared with `bucket_groups` as its
// nearly-identical seed pair.
pub(crate) const RUST_A: &str = "pub fn accumulate(limit: i64) -> i64 {\n\
                      let mut total = 0;\n\
                      let mut index = 0;\n\
                      while index < limit {\n\
                      total = total + index;\n\
                      index = index + 1;\n\
                      }\n\
                      total\n\
                      }\n";
pub(crate) const RUST_B: &str = "pub fn summate(bound: i64) -> i64 {\n\
                      let mut total = 0;\n\
                      let mut index = 0;\n\
                      while index < bound {\n\
                      total = total + index;\n\
                      index = index + 1;\n\
                      }\n\
                      total\n\
                      }\n";
// Two Dart files that are renamed (Type-2) clones of one function.
const DART_A: &str = "int accumulate(int limit) {\n\
                      var total = 0;\n\
                      var index = 0;\n\
                      while (index < limit) {\n\
                      total = total + index;\n\
                      index = index + 1;\n\
                      }\n\
                      return total;\n\
                      }\n";
const DART_B: &str = "int summate(int bound) {\n\
                      var acc = 0;\n\
                      var cursor = 0;\n\
                      while (cursor < bound) {\n\
                      acc = acc + cursor;\n\
                      cursor = cursor + 1;\n\
                      }\n\
                      return acc;\n\
                      }\n";

/// Writes a polyglot corpus: a renamed-clone Rust pair under `rust/`
/// and a renamed-clone Dart pair under `dart/`. Each language yields a
/// single-language cluster ([CONFIG-CROSS-LANGUAGE]).
fn write_polyglot_clones(dir: &Path) -> Result<()> {
    let rust = dir.join("rust");
    let dart = dir.join("dart");
    fs::create_dir_all(&rust)?;
    fs::create_dir_all(&dart)?;
    fs::write(rust.join("a.rs"), RUST_A)?;
    fs::write(rust.join("b.rs"), RUST_B)?;
    fs::write(dart.join("a.dart"), DART_A)?;
    fs::write(dart.join("b.dart"), DART_B)?;
    Ok(())
}

/// Runs `deslop` over a freshly-seeded polyglot corpus, returning the
/// rendered HTML report body. `extra` injects flags (e.g.
/// `--split-by-language`) before `--output`.
fn render_polyglot_html(tmp: &Path, extra: &[&str]) -> Result<String> {
    let scan_root = tmp.join("src");
    write_polyglot_clones(&scan_root)?;
    let out = outputs_under(tmp);
    let mut cmd = deslop_command(&scan_root, &tmp.join("report"))?;
    let _assertion = cmd
        .args(["--min-nodes", "8"])
        .args(extra.iter())
        .assert()
        .success();
    Ok(fs::read_to_string(&out.html)?)
}

// Implements [OUTPUT-HUMAN-HTML]: without the flag the report body is a
// single ranked "Duplicate groups" list with no per-language sections.
#[test]
fn html_report_is_one_ranked_list_by_default() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let html = render_polyglot_html(tmp.path(), &[])?;
    assert!(
        html.contains("<h2>Duplicate groups</h2>"),
        "default report keeps the single ranked list"
    );
    assert!(
        !html.contains("group(s)</h2>"),
        "no per-language section headings appear without the flag"
    );
    assert!(
        !html.contains("By language:"),
        "no per-language intro breakdown without the flag"
    );
    Ok(())
}

// Implements [OUTPUT-HUMAN-HTML-LANGUAGE-SECTIONS]: `--split-by-language`
// divides the body into one section per language with its own heading,
// and the flat heading disappears.
#[test]
fn html_report_splits_into_language_sections_via_flag() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let html = render_polyglot_html(tmp.path(), &["--split-by-language"])?;
    assert!(
        html.contains("<h2>Rust — "),
        "a Rust section heading is rendered"
    );
    assert!(
        html.contains("<h2>Dart — "),
        "a Dart section heading is rendered"
    );
    assert!(
        !html.contains("<h2>Duplicate groups</h2>"),
        "the flat heading is replaced by per-language sections"
    );
    assert!(
        html.contains("By language:"),
        "the intro gains a per-language breakdown"
    );
    Ok(())
}

// Implements [OUTPUT-HUMAN-HTML-LANGUAGE-SECTIONS]: `[report]
// split_by_language = true` in `.deslop.toml` enables the same split as
// the CLI flag.
#[test]
fn html_report_splits_into_language_sections_via_config() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    write_polyglot_clones(&scan_root)?;
    fs::write(
        scan_root.join(".deslop.toml"),
        "[report]\nsplit_by_language = true\n",
    )?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd.args(["--min-nodes", "8"]).assert().success();
    let html = fs::read_to_string(&out.html)?;
    assert!(
        html.contains("<h2>Rust — ") && html.contains("<h2>Dart — "),
        "config-driven split renders per-language sections"
    );
    Ok(())
}
