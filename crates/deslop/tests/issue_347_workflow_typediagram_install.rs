//! [CORPUS-CI] Pins issue #347: `.github/workflows/corpus.yml` compiled the
//! workspace without installing `typediagram`, so `deslop-core`'s `build.rs`
//! aborted with `spawnSync typediagram ENOENT` before a single corpus
//! repository was scanned — three scheduled runs of the real-repository
//! accuracy gate, three identical failures, zero measurements ever produced.
//!
//! The invariant: any workflow job that invokes `cargo` or `make` compiles
//! the workspace, and any workspace compile runs `deslop-core`'s `build.rs`,
//! which shells out to the `typediagram` CLI. Such a job must install
//! `typediagram` in an earlier step, pinned to exactly the version the
//! Makefile's `setup` target installs — the dependency-sync rule in
//! CLAUDE.md. Workflow files are parsed as YAML, never pattern-matched as
//! text.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use deslop_test_support::corpus::repo_root;
use yaml_rust2::{Yaml, YamlLoader};

/// A workflow job that compiles the Rust workspace.
struct CompilingJob {
    /// Workflow file name, e.g. `corpus.yml`.
    workflow: String,
    /// Job key under the workflow's `jobs:` mapping.
    job: String,
    /// Version from the job's `npm install -g typediagram@<version>` step,
    /// wherever in the job that step sits.
    install_pin: Option<String>,
    /// Whether the install step precedes the job's first compiling step.
    installed_before_first_build: bool,
}

/// Looks up `key` in a YAML mapping node.
fn field<'a>(node: &'a Yaml, key: &str) -> Option<&'a Yaml> {
    node.as_hash()
        .and_then(|hash| hash.get(&Yaml::String(key.to_owned())))
}

/// The shell command of a workflow step, when the step has `run:`.
fn step_command(step: &Yaml) -> Option<&str> {
    field(step, "run").and_then(Yaml::as_str)
}

/// Whether a step's command invokes a tool that compiles the workspace.
fn invokes_build_tool(command: &str) -> bool {
    command
        .split_whitespace()
        .any(|token| token == "cargo" || token == "make")
}

/// The version pin from an `npm install -g typediagram@<version>` command.
fn typediagram_pin(command: &str) -> Option<&str> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    (tokens.contains(&"npm") && tokens.contains(&"install"))
        .then(|| {
            tokens
                .iter()
                .find_map(|token| token.strip_prefix("typediagram@"))
        })
        .flatten()
}

/// Audits one job, returning its record when it compiles the workspace.
fn audit_job(workflow: &str, job_name: &str, job: &Yaml) -> Option<CompilingJob> {
    let steps = field(job, "steps").and_then(Yaml::as_vec)?;
    let mut install_pin = None;
    let mut installed_before_first_build = false;
    let mut build_seen = false;
    for step in steps {
        let Some(command) = step_command(step) else {
            continue;
        };
        if let Some(version) = typediagram_pin(command) {
            install_pin = Some(version.to_owned());
            installed_before_first_build = !build_seen;
        }
        build_seen = build_seen || invokes_build_tool(command);
    }
    build_seen.then(|| CompilingJob {
        workflow: workflow.to_owned(),
        job: job_name.to_owned(),
        install_pin,
        installed_before_first_build,
    })
}

/// Parses the first YAML document in `path`.
fn load_first_document(path: &Path) -> Result<Yaml> {
    let source = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    YamlLoader::load_from_str(&source)
        .with_context(|| format!("parsing {}", path.display()))?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("{} contains no YAML documents", path.display()))
}

/// Collects every compiling job in one workflow file.
fn compiling_jobs(path: &Path) -> Result<Vec<CompilingJob>> {
    let document = load_first_document(path)?;
    let workflow = path
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| anyhow!("{} has no UTF-8 file name", path.display()))?;
    let jobs = field(&document, "jobs")
        .and_then(Yaml::as_hash)
        .ok_or_else(|| anyhow!("{workflow} has no jobs mapping"))?;
    let mut found = Vec::new();
    for (name, job) in jobs {
        let job_name = name
            .as_str()
            .ok_or_else(|| anyhow!("{workflow} has a non-string job key"))?;
        if let Some(audited) = audit_job(workflow, job_name, job) {
            found.push(audited);
        }
    }
    Ok(found)
}

/// Every workflow file under `.github/workflows`, sorted for determinism.
fn workflow_paths(root: &Path) -> Result<Vec<PathBuf>> {
    let directory = root.join(".github").join("workflows");
    let entries =
        fs::read_dir(&directory).with_context(|| format!("listing {}", directory.display()))?;
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry
            .with_context(|| format!("reading an entry of {}", directory.display()))?
            .path();
        let is_workflow = path
            .extension()
            .is_some_and(|extension| extension == "yml" || extension == "yaml");
        if is_workflow {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// Every `typediagram@<version>` pin in the Makefile, in file order.
fn makefile_pins(root: &Path) -> Result<Vec<String>> {
    let makefile = root.join("Makefile");
    let source =
        fs::read_to_string(&makefile).with_context(|| format!("reading {}", makefile.display()))?;
    Ok(source
        .split_whitespace()
        .filter_map(|token| token.strip_prefix("typediagram@"))
        .map(str::to_owned)
        .collect())
}

/// Asserts `version` is an exact dotted numeric pin, e.g. `0.11.0` — not a
/// tag like `latest` that would satisfy sync while abandoning pinning.
fn assert_exact_version(version: &str, source: &str) {
    assert!(
        version.split('.').count() >= 2
            && version
                .split('.')
                .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())),
        "{source} pins typediagram@{version}, which is not an exact numeric version"
    );
}

/// Asserts one compiling job installs the expected typediagram pin first.
fn assert_job_installs(job: &CompilingJob, expected_pin: &str) {
    let CompilingJob {
        workflow,
        job: job_name,
        install_pin,
        installed_before_first_build,
    } = job;
    assert!(
        install_pin.is_some(),
        "{workflow} job `{job_name}` compiles the workspace but never installs typediagram; \
         deslop-core's build.rs aborts exactly as in issue #347"
    );
    assert!(
        *installed_before_first_build,
        "{workflow} job `{job_name}` installs typediagram only after its first cargo/make step; \
         the build has already failed by then (#347)"
    );
    let pin = install_pin.as_deref().unwrap_or_default();
    assert_eq!(
        pin, expected_pin,
        "{workflow} job `{job_name}` pins typediagram@{pin} but the Makefile setup target pins \
         typediagram@{expected_pin}; the dependency-sync rule requires one version everywhere"
    );
    assert_exact_version(pin, workflow);
}

/// Asserts the audit still recognises the jobs known to compile the
/// workspace: the corpus gate, the four `ci.yml` jobs, and the release
/// build. Without this the predicate could go blind and the test would pass
/// while asserting nothing.
fn assert_expected_coverage(jobs: &[CompilingJob]) {
    let found: Vec<(&str, &str)> = jobs
        .iter()
        .map(|job| (job.workflow.as_str(), job.job.as_str()))
        .collect();
    assert!(
        found.contains(&("corpus.yml", "corpus")),
        "the corpus.yml `corpus` job was not recognised as compiling the workspace; the issue \
         #347 regression guard has gone blind. found={found:?}"
    );
    assert!(
        jobs.iter().filter(|job| job.workflow == "ci.yml").count() >= 4,
        "expected at least four ci.yml jobs to compile the workspace. found={found:?}"
    );
    assert!(
        jobs.iter().any(|job| job.workflow == "release.yml"),
        "expected the release.yml build job to compile the workspace. found={found:?}"
    );
}

/// [CORPUS-CI] Issue #347: every workflow job that compiles the workspace
/// installs the Makefile-pinned typediagram before its first build step.
#[test]
fn workflows_that_compile_the_workspace_install_pinned_typediagram_first() -> Result<()> {
    let root = repo_root();
    let pins = makefile_pins(&root)?;
    let expected_pin = pins
        .first()
        .ok_or_else(|| anyhow!("the Makefile setup target no longer pins typediagram"))?;
    assert!(
        pins.iter().all(|pin| pin == expected_pin),
        "the Makefile pins multiple typediagram versions: {pins:?}"
    );
    assert_exact_version(expected_pin, "Makefile");
    let mut jobs = Vec::new();
    for path in workflow_paths(&root)? {
        jobs.extend(compiling_jobs(&path)?);
    }
    assert_expected_coverage(&jobs);
    for job in &jobs {
        assert_job_installs(job, expected_pin);
    }
    Ok(())
}
