//! Mechanical parameter naming, coalescing, and default computation
//! ([AUTOFIX-MERGE-NAMES], [AUTOFIX-MERGE-DEFAULTS]).

use std::collections::{HashMap, HashSet};

use crate::{
    refactor::merge::{gate::Hole, safety::HoleRole},
    wire_generated::MergeParameter,
};

/// Maximum parameter arity after coalescing (gate 4c — "a handful").
const MAX_PARAMS: usize = 4;

/// One parameter slot with the gate holes it absorbs (the
/// [AUTOFIX-MERGE-ANTIUNIFY] store/coalesce rule: identical per-site
/// tuples reuse one variable).
#[derive(Debug)]
pub struct ParameterSlot {
    /// Indexes into the gate's hole list served by this slot.
    pub hole_indexes: Vec<usize>,
    /// The finished wire parameter.
    pub parameter: MergeParameter,
}

/// Derives the coalesced, named, typed, defaulted parameter list.
///
/// # Errors
///
/// `Err` carries the routing reason (arity budget).
pub fn derive(
    holes: &[Hole],
    roles: &[HoleRole],
    supports_defaults: bool,
) -> Result<Vec<ParameterSlot>, String> {
    let mut slots = coalesce(holes, roles);
    if slots.len() > MAX_PARAMS {
        return Err(format!(
            "{} parameters exceed the arity budget of {MAX_PARAMS}",
            slots.len()
        ));
    }
    assign_names(&mut slots, holes);
    if supports_defaults {
        assign_defaults(&mut slots);
    }
    Ok(slots)
}

/// Groups parameter holes by their per-site text tuple (the coalesce
/// store), preserving first-appearance order.
fn coalesce(holes: &[Hole], roles: &[HoleRole]) -> Vec<ParameterSlot> {
    let mut by_tuple: HashMap<Vec<String>, usize> = HashMap::new();
    let mut slots: Vec<ParameterSlot> = Vec::new();
    for (index, (hole, role)) in holes.iter().zip(roles).enumerate() {
        let HoleRole::Parameter { type_name } = role else {
            continue;
        };
        let tuple: Vec<String> = hole.per_site.iter().map(|site| site.text.clone()).collect();
        if let Some(slot_index) = by_tuple.get(&tuple) {
            if let Some(slot) = slots.get_mut(*slot_index) {
                slot.hole_indexes.push(index);
            }
            continue;
        }
        let _stored = by_tuple.insert(tuple.clone(), slots.len());
        slots.push(ParameterSlot {
            hole_indexes: vec![index],
            parameter: MergeParameter {
                name: String::new(),
                type_name: type_name.clone(),
                is_thunk: false,
                is_required: true,
                default_value: None,
                per_site_arguments: tuple,
            },
        });
    }
    slots
}

/// Names each slot: the modal candidate identifier when one exists and
/// stays unique, else positional (`arg0`, `arg1`, …)
/// ([AUTOFIX-MERGE-NAMES]).
fn assign_names(slots: &mut [ParameterSlot], holes: &[Hole]) {
    let mut taken: HashSet<String> = HashSet::new();
    for (position, slot) in slots.iter_mut().enumerate() {
        let candidate = slot
            .hole_indexes
            .first()
            .and_then(|index| holes.get(*index))
            .and_then(modal_identifier)
            .filter(|name| !taken.contains(name));
        let name = candidate.unwrap_or_else(|| format!("arg{position}"));
        let _new = taken.insert(name.clone());
        slot.parameter.name = name;
    }
}

/// The strictly-most-frequent per-site identifier of a hole, when it
/// is a single valid identifier.
fn modal_identifier(hole: &Hole) -> Option<String> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for site in &hole.per_site {
        *counts.entry(site.text.as_str()).or_insert(0) = counts
            .get(site.text.as_str())
            .copied()
            .unwrap_or_default()
            .saturating_add(1);
    }
    let best = counts.iter().max_by_key(|(_, count)| **count)?;
    let unique_mode = counts.values().filter(|count| *count == best.1).count() == 1;
    (unique_mode && is_identifier(best.0)).then(|| (*best.0).to_owned())
}

/// ASCII identifier check for generated parameter names.
fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|rest| rest.is_ascii_alphanumeric() || rest == '_')
}

/// Marks the maximal trailing run of default-eligible slots optional
/// ([AUTOFIX-MERGE-DEFAULTS]): a slot is eligible when at least two
/// sites — and all but at most one — share one value. Because check F
/// rewrites every site atomically, defaults are a readability win only.
fn assign_defaults(slots: &mut [ParameterSlot]) {
    for slot in slots.iter_mut().rev() {
        let Some(default) = modal_default(&slot.parameter.per_site_arguments) else {
            break;
        };
        slot.parameter.default_value = Some(default);
        slot.parameter.is_required = false;
    }
}

/// The dominant value of a slot when all but at most one site share it
/// and at least two sites do.
fn modal_default(arguments: &[String]) -> Option<String> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for argument in arguments {
        *counts.entry(argument.as_str()).or_insert(0) = counts
            .get(argument.as_str())
            .copied()
            .unwrap_or_default()
            .saturating_add(1);
    }
    let (value, count) = counts.into_iter().max_by_key(|(_, count)| *count)?;
    (count >= 2 && count.saturating_add(1) >= arguments.len()).then(|| value.to_owned())
}
