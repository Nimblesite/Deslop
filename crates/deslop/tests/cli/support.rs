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

/// Appends `.<ext>` to `base` by cloning and replacing the file name.
fn with_ext(base: &Path, ext: &str) -> PathBuf {
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

/// Copies every top-level entry in `src` into a freshly created `dst`.
/// Used by tests that need a mutable scan root seeded from an
/// immutable fixture (cache/embedding tests write siblings next to the
/// sources).
pub(crate) fn seed_scan_root(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let _bytes = fs::copy(entry.path(), dst.join(entry.file_name()))?;
    }
    Ok(())
}

/// Collects every `deslop-*.log` file sitting in `dir`. The default
/// logging path writes a timestamped file next to the report; tests
/// need to locate it without hardcoding the stamp.
pub(crate) fn find_timestamped_logs(dir: &Path) -> Result<Vec<PathBuf>> {
    let matches = fs::read_dir(dir)?
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
