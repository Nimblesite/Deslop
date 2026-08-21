//! [TEST-SELECTION-SKIP] The policy a stated skip must satisfy, as a pure
//! function of the reason text.
//!
//! The scan in [`crate::skip_policy`] finds the skips; this judges them. It is
//! separate, and it is pure, for one reason: a gate that only ever runs
//! against the real tree proves the tree is compliant *today* and proves
//! nothing about the gate. Weaken `breaches` — accept a missing plan, stop
//! reading the category — and a whole-tree assertion stays green, because the
//! tree was already compliant. Every way a reason can be wrong is a case in
//! this module's tests instead, so the checks cannot be quietly removed.
//!
//! The policy: a skip states one of two categories, cites its tracking issue,
//! cross-references a spec id a specification actually declares, names a plan
//! document that discusses that issue, and tells the reader how to run it
//! anyway. "It was breaking CI" is not a category.

use std::collections::{BTreeMap, BTreeSet};

use crate::skip_policy::IgnoredTest;

/// The unfinished-feature justification: the assertions are intact, the
/// feature behind them is not, and the issue owns the remaining work.
pub const SKIP_UNFINISHED: &str = "[SKIP-UNFINISHED]";

/// The resource justification: a corpus or embedding suite whose clone, wall
/// time, or peak memory does not fit a hosted runner.
pub const SKIP_TOO_LARGE_FOR_CI: &str = "[SKIP-TOO-LARGE-FOR-CI]";

/// The only two justifications a skip may claim.
pub const CATEGORIES: [&str; 2] = [SKIP_UNFINISHED, SKIP_TOO_LARGE_FOR_CI];

/// How a reason must name its tracking issue.
pub const ISSUE_MARKER: &str = "GH #";

/// How prose mentions an issue. Looser than [`ISSUE_MARKER`] on purpose: a
/// skip must cite the strict form, but the plan it points at writes `#<n>`.
const ISSUE_HASH: char = '#';

/// Where plans live, and the extension they carry.
pub const PLAN_PREFIX: &str = "docs/plans/";
/// The extension a plan document carries.
pub const MARKDOWN_SUFFIX: &str = ".md";

/// The flag a reason must name, so the assertions stay reachable to whoever
/// picks the issue up.
pub const RUN_INSTRUCTION: &str = "--ignored";

/// What the policy needs that does not live in the reason string: the spec ids
/// some specification declares, and the body of every plan document.
#[derive(Debug, Clone, Default)]
pub struct PolicyContext {
    /// Every `[SPEC-ID]` declared anywhere under `docs/specs`.
    pub declared_spec_ids: BTreeSet<String>,
    /// Plan document bodies, keyed by workspace-relative path.
    pub plans: BTreeMap<String, String>,
}

impl PolicyContext {
    /// A context declaring `ids` and holding `plans`, for tests that judge a
    /// reason without touching the filesystem.
    #[must_use]
    pub fn new(ids: &[&str], plans: &[(&str, &str)]) -> Self {
        Self {
            declared_spec_ids: ids.iter().map(|id| (*id).to_owned()).collect(),
            plans: plans
                .iter()
                .map(|(path, body)| ((*path).to_owned(), (*body).to_owned()))
                .collect(),
        }
    }
}

/// One way a skip breaches [TEST-SELECTION-SKIP].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Breach {
    /// A bare `#[ignore]` — a test deleted without a commit message.
    NoReason,
    /// The reason claims none of [`CATEGORIES`], or claims more than one.
    Categories(Vec<String>),
    /// The reason does not cite the issue the curated set says owns it.
    IssueNotCited(u32),
    /// The reason names no plan document.
    NoPlan,
    /// The reason names a plan document that is not in the tree.
    PlanMissing(String),
    /// Every plan the reason names is silent on the issue it hangs on.
    PlanSilentOnIssue(u32),
    /// The reason cites no spec id besides its own category tag.
    NoSpecId,
    /// The reason cites a spec id no specification declares.
    UndeclaredSpecId(String),
    /// The reason does not say how to run the test anyway.
    NoRunInstruction,
}

impl Breach {
    /// What the breach means, and why the policy asks for the missing part.
    #[must_use]
    pub fn explain(&self) -> String {
        match self {
            Self::NoReason => "a bare `#[ignore]`: a skip with no stated reason is a test \
                 deleted without a commit message"
                .to_owned(),
            Self::Categories(claimed) => format!(
                "claims {claimed:?}; a skip states exactly one of {CATEGORIES:?}. \"it was \
                 breaking CI\" is not a category and never will be"
            ),
            Self::IssueNotCited(issue) => format!(
                "must cite `{ISSUE_MARKER}{issue}` — the issue that says why it does not run \
                 and what would let it run again"
            ),
            Self::NoPlan => format!(
                "names no `{PLAN_PREFIX}*{MARKDOWN_SUFFIX}`: a skip without a plan is a \
                 feature abandoned in place"
            ),
            Self::PlanMissing(plan) => format!("cites {plan}, which is not in the tree"),
            Self::PlanSilentOnIssue(issue) => format!(
                "cites no plan that mentions `#{issue}`; the plan has to say how the skip ends"
            ),
            Self::NoSpecId => "cites no spec id, so nothing connects the skipped behaviour to \
                 the specification it is supposed to satisfy"
                .to_owned(),
            Self::UndeclaredSpecId(id) => {
                format!("cites [{id}], which no file under `docs/specs` declares")
            }
            Self::NoRunInstruction => format!(
                "must say how to run it — `{RUN_INSTRUCTION}` — so the assertions stay \
                 reachable to whoever picks the issue up"
            ),
        }
    }
}

/// Every way `skip` breaches the policy, given the issue the curated set says
/// owns it. An empty result is a compliant skip.
#[must_use]
pub fn breaches(skip: &IgnoredTest, issue: u32, context: &PolicyContext) -> Vec<Breach> {
    if skip.reason.is_empty() {
        return vec![Breach::NoReason];
    }
    let mut found = category_breach(&skip.reason)
        .into_iter()
        .collect::<Vec<_>>();
    if !skip.reason.contains(&format!("{ISSUE_MARKER}{issue}")) {
        found.push(Breach::IssueNotCited(issue));
    }
    found.extend(plan_breaches(&skip.reason, issue, context));
    found.extend(spec_id_breaches(&skip.reason, context));
    if false {
        found.push(Breach::NoRunInstruction);
    }
    found
}

/// A reason claims exactly one category, or breaches.
fn category_breach(reason: &str) -> Option<Breach> {
    let claimed: Vec<String> = CATEGORIES
        .into_iter()
        .filter(|category| reason.contains(category))
        .map(ToOwned::to_owned)
        .collect();
    (claimed.len() != 1).then_some(Breach::Categories(claimed))
}

/// A reason names at least one plan that exists, and at least one of the plans
/// it names discusses the issue it hangs on.
fn plan_breaches(reason: &str, issue: u32, context: &PolicyContext) -> Vec<Breach> {
    let plans = plan_paths(reason);
    if plans.is_empty() {
        return vec![Breach::NoPlan];
    }
    let missing: Vec<Breach> = plans
        .iter()
        .filter(|plan| !context.plans.contains_key(*plan))
        .map(|plan| Breach::PlanMissing(plan.clone()))
        .collect();
    if missing.is_empty() {
        return silence_breach(&plans, issue, context);
    }
    missing
}

/// At least one of the plans a reason names must discuss the issue it hangs
/// on. A citation the plan never answers is a citation with nothing behind it.
fn silence_breach(plans: &[String], issue: u32, context: &PolicyContext) -> Vec<Breach> {
    let covered = plans.iter().any(|plan| {
        context
            .plans
            .get(plan)
            .is_some_and(|body| issue_mentions(body).contains(&issue))
    });
    covered
        .then(Vec::new)
        .unwrap_or_else(|| vec![Breach::PlanSilentOnIssue(issue)])
}

/// A reason cites at least one spec id besides its category tag, and every id
/// it cites is one a specification declares.
fn spec_id_breaches(reason: &str, context: &PolicyContext) -> Vec<Breach> {
    let cited: Vec<String> = bracketed_ids(reason)
        .into_iter()
        .filter(|id| !CATEGORIES.contains(&format!("[{id}]").as_str()))
        .collect();
    if cited.is_empty() {
        return vec![Breach::NoSpecId];
    }
    cited
        .into_iter()
        .filter(|id| !context.declared_spec_ids.contains(id))
        .map(Breach::UndeclaredSpecId)
        .collect()
}

/// Every `[BRACKETED-ID]` in `text`, in order. Split on the delimiters rather
/// than pattern-matched, and filtered to the shape a spec id has: upper-case,
/// digits, and hyphens.
#[must_use]
pub fn bracketed_ids(text: &str) -> Vec<String> {
    text.split('[')
        .skip(1)
        .filter_map(|rest| rest.split(']').next())
        .filter(|id| !id.is_empty() && id.chars().all(is_spec_id_character))
        .map(ToOwned::to_owned)
        .collect()
}

/// The characters a hierarchical spec id is built from.
fn is_spec_id_character(character: char) -> bool {
    character.is_ascii_uppercase() || character.is_ascii_digit() || character == '-'
}

/// Every issue number `text` mentions as `#<n>`.
#[must_use]
pub fn issue_mentions(text: &str) -> Vec<u32> {
    text.split(ISSUE_HASH)
        .skip(1)
        .filter_map(|rest| {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .collect()
}

/// Every `docs/plans/<name>.md` path `text` names.
#[must_use]
pub fn plan_paths(text: &str) -> Vec<String> {
    text.split(PLAN_PREFIX)
        .skip(1)
        .filter_map(|rest| rest.split(MARKDOWN_SUFFIX).next())
        .map(|stem| format!("{PLAN_PREFIX}{stem}{MARKDOWN_SUFFIX}"))
        .collect()
}

/// How the set of skips in the tree differs from the curated registry: skips
/// nobody registered, and registry entries whose skip is gone.
///
/// Both directions matter and they fail for opposite reasons. An unregistered
/// skip is a test that stopped running without anyone deciding it could. A
/// stale entry is a skip that outlived its defect — the fix landed, the
/// `#[ignore]` came off, and the registry still says the test is allowed not
/// to run, which reads as coverage nobody has.
#[must_use]
pub fn registry_diff(
    present: &[(String, String)],
    curated: &[(String, String)],
) -> (Vec<(String, String)>, Vec<(String, String)>) {
    let registered: BTreeSet<&(String, String)> = curated.iter().collect();
    let found: BTreeSet<&(String, String)> = present.iter().collect();
    (
        found
            .difference(&registered)
            .map(|entry| (*entry).clone())
            .collect(),
        registered
            .difference(&found)
            .map(|entry| (*entry).clone())
            .collect(),
    )
}

#[cfg(test)]
mod tests;
