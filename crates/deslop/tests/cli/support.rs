pub(crate) use anyhow::{Context, Result};
pub(crate) use assert_cmd::Command;
pub(crate) use predicates::str::contains;
pub(crate) use serde_json::Value;
pub(crate) use std::{fs, path::Path, path::PathBuf};

pub(crate) fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Runs the binary in `<tmp>` with `--output <tmp>/report`, returning
/// the three on-disk paths the CLI should have written.
pub(crate) struct RunOutputs {
    /// Path to `<tmp>/report.json`.
    pub(crate) json: PathBuf,
    /// Path to `<tmp>/report.txt`.
    pub(crate) txt: PathBuf,
    /// Path to `<tmp>/report.html`.
    pub(crate) html: PathBuf,
}

/// Renders the three output paths for an `--output <dir>/report` layout.
pub(crate) fn outputs_under(dir: &Path) -> RunOutputs {
    let base = dir.join("report");
    RunOutputs {
        json: with_ext(&base, "json"),
        txt: with_ext(&base, "txt"),
        html: with_ext(&base, "html"),
    }
}

pub(crate) fn deslop_command(scan_root: &Path, output_prefix: &Path) -> Result<Command> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _cmd = cmd.arg(scan_root).arg("--output").arg(output_prefix);
    Ok(cmd)
}

/// Builds a command scanning the shared fixture `name`, with the
/// fingerprint cache disabled.
///
/// The cache is on by default ([PIPELINE-INCREMENTAL]) and always lands
/// in the *scan root* ([OUTPUT-DIR]) — `--output` cannot redirect it,
/// because the LSP and MCP have to find it from the scan root alone.
/// A fixture directory is checked into the repo and shared by the whole
/// suite, so running a test must never write into one. Tests that
/// actually exercise the cache seed a mutable scan root with
/// [`seed_scan_root`] instead of reaching for a fixture directly.
pub(crate) fn fixture_command(name: &str, output_prefix: &Path) -> Result<Command> {
    let mut cmd = deslop_command(&fixture(name), output_prefix)?;
    let _cmd = cmd.arg("--no-incremental");
    Ok(cmd)
}

/// Appends `.<ext>` to `base` by cloning and replacing the file name.
pub(crate) fn with_ext(base: &Path, ext: &str) -> PathBuf {
    deslop_test_support::with_ext(base, ext)
}

/// Copies every top-level entry in `src` into a freshly created `dst`.
/// Used by tests that need a mutable scan root seeded from an
/// immutable fixture (cache/embedding tests write siblings next to the
/// sources).
pub(crate) fn seed_scan_root(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        // Source files only. A fixture directory can pick up a nested
        // `.deslop/` output directory ([OUTPUT-DIR]) from a stray run, and
        // `fs::copy` on a directory fails outright — seeding must not be
        // hostage to whatever else happens to be sitting there.
        if !entry.file_type()?.is_file() {
            continue;
        }
        let _bytes = fs::copy(entry.path(), dst.join(entry.file_name()))?;
    }
    Ok(())
}

/// Collects every `deslop-*.log` file the CLI wrote for reports based
/// in `report_dir`. Logs land in that directory's `logs/` subdirectory
/// ([OUTPUT-DIR]); tests need to locate them without hardcoding the
/// timestamp. An absent `logs/` means no log file was written — exactly
/// what `--log-to-console` must produce — so it yields an empty vec
/// instead of an error.
pub(crate) fn find_timestamped_logs(report_dir: &Path) -> Result<Vec<PathBuf>> {
    let logs_dir = report_dir.join("logs");
    if !logs_dir.is_dir() {
        return Ok(Vec::new());
    }
    let matches = fs::read_dir(&logs_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("deslop-")
                        && Path::new(name)
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("log"))
                })
        })
        .collect();
    Ok(matches)
}

// Implements [CLI-INVOCATION-VERSION]: `deslop --version` prints the

pub(crate) fn write_clone_pair(dir: &Path) -> Result<u64> {
    fs::create_dir_all(dir)?;
    let alpha = "namespace Alpha\n\
                 {\n\
                 public class Processor\n\
                 {\n\
                 public int Compute(int input)\n\
                 {\n\
                 if (input < 0) { return 0; }\n\
                 int total = 0;\n\
                 for (int i = 0; i < input; i = i + 1) { total = total + i; }\n\
                 return total;\n\
                 }\n\
                 }\n\
                 }\n";
    let beta = "namespace Beta\n\
                {\n\
                public class Summer\n\
                {\n\
                public int Run(int limit)\n\
                {\n\
                if (limit < 0) { return 0; }\n\
                int acc = 0;\n\
                for (int j = 0; j < limit; j = j + 1) { acc = acc + j; }\n\
                return acc;\n\
                }\n\
                }\n\
                }\n";
    fs::write(dir.join("Alpha.cs"), alpha)?;
    fs::write(dir.join("Beta.cs"), beta)?;
    // Each file = 13 newline-terminated lines. Two files => 26.
    Ok(26)
}

/// Returns the parsed JSON report from a successful run.
pub(crate) fn read_json_report(path: &Path) -> Result<serde_json::Value> {
    let body = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&body)?)
}

/// The JSON object at `key` in `value`, erroring with `context` when the key
/// is absent or its value is not an object. Centralises the
/// `get(key).and_then(as_object).ok_or_else(...)` idiom while preserving each
/// call site's bespoke error message.
pub(crate) fn object_field<'a>(
    value: &'a Value,
    key: &str,
    context: &str,
) -> Result<&'a serde_json::Map<String, Value>> {
    value
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("{context}"))
}

/// Looks up a named field on `value`; returns `Value::Null` when the
/// field is absent so callers get a deterministic `!=` instead of a
/// panic ([TESTS-NO-INDEXING]).
pub(crate) fn field<'a>(value: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    value.get(name).unwrap_or(&serde_json::Value::Null)
}

/// Shortcut for `field(field(value, "metrics"), key)`.
pub(crate) fn metric_field<'a>(report: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    field(field(report, "metrics"), key)
}

/// Shortcut for `field(field(field(value, "metrics"), "threshold"), key)`.
pub(crate) fn threshold_field<'a>(
    report: &'a serde_json::Value,
    key: &str,
) -> &'a serde_json::Value {
    field(metric_field(report, "threshold"), key)
}

// Implements [METRICS-REPO]: empty corpus yields zero metrics. Still a
