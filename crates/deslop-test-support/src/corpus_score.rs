//! [CORPUS-SCORE] Scores rendered reports against the clone registers and
//! renders the corpus scorecard. Spec: `docs/specs/corpus.md` [CORPUS-SCORE].
//!
//! A register is independent ground truth: pairs a judge classified CLEARLY IN
//! or CLEARLY OUT while isolated from this codebase (`docs/specs/corpus.md`
//! [CORPUS-REGISTER]). Both verdicts read **one predicate in opposite
//! directions**: an entry is *matched* when some published cluster shows
//! visible occurrences overlapping every listed range. A matched CLEARLY IN is
//! correct and an unmatched one is a **false negative**; a matched CLEARLY OUT
//! is a **false positive** and an unmatched one is correct.
//!
//! Overlap, never exact line equality — that is what keeps the assertion
//! non-fragile across extent drift, rank movement and occurrence-count changes.
//!
//! Every figure the scorecard prints is computed here. Nothing downstream
//! recomputes a count, a percentage or a delta.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod gate;
pub mod render;

/// The two judged verdicts. NOT CLEAR is recorded in a register and scores
/// nothing, by design.
pub const CLEARLY_IN: &str = "clearly_in";
/// See [`CLEARLY_IN`].
pub const CLEARLY_OUT: &str = "clearly_out";

/// Default gate on defects: none tolerated unless the config records one.
pub const DEFAULT_MAXIMUM_DEFECTS: usize = 0;
/// Percentage base, named so the calculation reads as one.
const PERCENT: f64 = 100.0;

/// What one scan cost, as measured around the engine process itself.
///
/// Cost is reported beside the score and never folded into it: a slower engine
/// that finds the same pairs has not become less accurate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunCost {
    /// Wall-clock milliseconds the scan process took.
    pub elapsed_ms: u64,
    /// Peak resident set in mebibytes, absent when the platform could not
    /// measure it. An absent figure is printed as absent, never as zero.
    #[serde(default)]
    pub peak_rss_mb: Option<u64>,
    /// User + system CPU seconds, absent when unmeasured.
    #[serde(default)]
    pub cpu_seconds: Option<f64>,
    /// The exact binary that produced the numbers. A figure whose producing
    /// binary is unidentified is not comparable.
    pub binary_sha256: String,
}

/// One judged range, written `path:startLine-endLine` in a register.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Range {
    /// Repository-relative path of the file the judge read.
    pub path: String,
    /// First line of the judged region, 1-based and inclusive.
    pub start: u32,
    /// Last line of the judged region, inclusive.
    pub end: u32,
}

impl Range {
    /// Parses `path:start-end`.
    ///
    /// # Errors
    ///
    /// Returns an error when the text is not that shape, or names an empty or
    /// inverted span — either would score an entry that describes nothing.
    pub fn parse(text: &str) -> Result<Self> {
        let (path, span) = text
            .rsplit_once(':')
            .ok_or_else(|| anyhow!("register range is not `path:start-end`: {text}"))?;
        let (start, end) = span
            .split_once('-')
            .ok_or_else(|| anyhow!("register range has no line span: {text}"))?;
        let start: u32 = start.trim().parse().context("range start is not a line")?;
        let end: u32 = end.trim().parse().context("range end is not a line")?;
        if start < 1 || end < start {
            return Err(anyhow!("register range is empty or inverted: {text}"));
        }
        Ok(Self {
            path: path.to_owned(),
            start,
            end,
        })
    }
}

/// One register entry after scoring.
#[derive(Debug, Clone, Serialize)]
pub struct ScoredEntry {
    /// [`CLEARLY_IN`] or [`CLEARLY_OUT`].
    pub verdict: String,
    /// The judge's reason, carried through so a breach explains itself.
    pub why: String,
    /// The ranges as the register wrote them.
    pub occurrences: Vec<String>,
    /// Whether a published cluster showed every range together.
    pub matched: bool,
    /// The cluster that matched, when one did.
    pub cluster: Option<String>,
    /// Whether the engine got this entry right.
    pub correct: bool,
}

impl ScoredEntry {
    /// Whether this entry is a false negative: a CLEARLY IN nobody reported.
    #[must_use]
    pub fn is_false_negative(&self) -> bool {
        self.verdict == CLEARLY_IN && !self.correct
    }

    /// Whether this entry is a false positive: a CLEARLY OUT that was reported.
    #[must_use]
    pub fn is_false_positive(&self) -> bool {
        self.verdict == CLEARLY_OUT && !self.correct
    }
}

/// One repository's standing against its register for one engine.
#[derive(Debug, Clone, Serialize)]
pub struct RepoScore {
    /// Repository name, matching the register file stem.
    pub name: String,
    /// The commit the register was judged at and the report was scanned at.
    pub sha: String,
    /// CLEARLY IN entries, and how many the engine found.
    pub clearly_in_total: usize,
    /// See [`Self::clearly_in_total`].
    pub clearly_in_found: usize,
    /// CLEARLY OUT entries, and how many the engine correctly stayed silent on.
    pub clearly_out_total: usize,
    /// See [`Self::clearly_out_total`].
    pub clearly_out_absent: usize,
    /// Judged entries the engine got wrong, in each direction.
    pub false_negatives: usize,
    /// See [`Self::false_negatives`].
    pub false_positives: usize,
    /// Judged entries in total, and how many were answered correctly.
    pub judged: usize,
    /// See [`Self::judged`].
    pub correct: usize,
    /// `100 * correct / judged`, absent when nothing is judged. A register with
    /// no entries scores nothing rather than scoring perfectly.
    pub score_percent: Option<f64>,
    /// Clusters the report published. Description, never part of the score.
    pub clusters_total: usize,
    /// Every judged entry, so a breach can name the pair it broke.
    pub entries: Vec<ScoredEntry>,
}

/// Whether the report shows this occurrence to a reader. A hidden occurrence
/// is not something the user was told, so it can neither satisfy a CLEARLY IN
/// nor breach a CLEARLY OUT.
fn is_visible(occurrence: &Value) -> bool {
    !occurrence
        .get("hidden")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// Whether a published occurrence covers any line of a judged range.
fn overlaps(occurrence: &Value, range: &Range) -> bool {
    let line = |field: &str| occurrence.get(field).and_then(Value::as_u64).unwrap_or(0);
    occurrence.get("path").and_then(Value::as_str) == Some(range.path.as_str())
        && line("start_line") <= u64::from(range.end)
        && line("end_line") >= u64::from(range.start)
}

/// The visible occurrences of one cluster.
fn visible(cluster: &Value) -> Vec<&Value> {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .map(|list| list.iter().filter(|o| is_visible(o)).collect())
        .unwrap_or_default()
}

/// The clusters a report published.
fn clusters(report: &Value) -> &[Value] {
    report
        .get("clusters")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
}

/// The id of the cluster that shows every range of one entry together, if any.
fn matching_cluster(report: &Value, ranges: &[Range]) -> Option<String> {
    clusters(report)
        .iter()
        .find(|cluster| {
            let shown = visible(cluster);
            ranges
                .iter()
                .all(|range| shown.iter().any(|o| overlaps(o, range)))
        })
        .and_then(|cluster| cluster.get("id").and_then(Value::as_str))
        .map(ToOwned::to_owned)
}

/// The entries of one verdict list, empty when the key is absent.
fn list<'a>(register: &'a Value, verdict: &str) -> &'a [Value] {
    register
        .get(verdict)
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
}

/// A string field, blank when absent.
fn text(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// The ranges one entry names, as the register wrote them.
///
/// A range that is not a string is refused rather than skipped: silently
/// dropping one would score the entry against fewer regions than the judge
/// read, which can only make the engine look better than it is.
fn written_ranges(entry: &Value) -> Result<Vec<String>> {
    list(entry, "occurrences")
        .iter()
        .map(|range| {
            range
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| anyhow!("register range is not a string: {range}"))
        })
        .collect()
}

/// Scores one register entry against one report.
fn score_entry(report: &Value, entry: &Value, verdict: &str) -> Result<ScoredEntry> {
    let written = written_ranges(entry)?;
    let ranges = written
        .iter()
        .map(|range| Range::parse(range))
        .collect::<Result<Vec<_>>>()?;
    let cluster = matching_cluster(report, &ranges);
    let matched = cluster.is_some();
    Ok(ScoredEntry {
        verdict: verdict.to_owned(),
        why: text(entry, "why"),
        occurrences: written,
        matched,
        cluster,
        correct: if verdict == CLEARLY_IN {
            matched
        } else {
            !matched
        },
    })
}

/// Scores every entry of one verdict list against one report.
fn score_list(report: &Value, register: &Value, verdict: &str) -> Result<Vec<ScoredEntry>> {
    list(register, verdict)
        .iter()
        .map(|entry| score_entry(report, entry, verdict))
        .collect()
}

/// Scores one rendered report against one register.
///
/// # Errors
///
/// Returns an error when a register range is malformed — an entry that names
/// nothing must fail loudly rather than silently score as correct.
pub fn score_repo(name: &str, register: &Value, report: &Value) -> Result<RepoScore> {
    let mut entries = score_list(report, register, CLEARLY_IN)?;
    entries.extend(score_list(report, register, CLEARLY_OUT)?);

    let count = |verdict: &str| entries.iter().filter(|e| e.verdict == verdict).count();
    let clearly_in_total = count(CLEARLY_IN);
    let clearly_out_total = count(CLEARLY_OUT);
    let false_negatives = entries.iter().filter(|e| e.is_false_negative()).count();
    let false_positives = entries.iter().filter(|e| e.is_false_positive()).count();
    let judged = clearly_in_total.saturating_add(clearly_out_total);
    let correct = judged
        .saturating_sub(false_negatives)
        .saturating_sub(false_positives);

    Ok(RepoScore {
        name: name.to_owned(),
        sha: text(register, "sha"),
        clearly_in_total,
        clearly_in_found: clearly_in_total.saturating_sub(false_negatives),
        clearly_out_total,
        clearly_out_absent: clearly_out_total.saturating_sub(false_positives),
        false_negatives,
        false_positives,
        judged,
        correct,
        score_percent: percent(correct, judged),
        clusters_total: clusters(report).len(),
        entries,
    })
}

/// `100 * correct / judged`, or `None` when nothing is judged.
///
/// An unjudged register must never read as a perfect score: the difference
/// between "answered every question right" and "was asked nothing" is the
/// whole point of publishing the denominator beside the figure.
#[must_use]
pub fn percent(correct: usize, judged: usize) -> Option<f64> {
    (judged > 0).then(|| PERCENT * as_f64(correct) / as_f64(judged))
}

/// Widening that keeps the lossy cast in one reviewed place.
fn as_f64(value: usize) -> f64 {
    u32::try_from(value).map_or(f64::from(u32::MAX), f64::from)
}

#[cfg(test)]
mod tests;
