//! Saturating counter folds shared by the pipeline's shard tallies.
//!
//! Every parallel stage splits its work across shards and folds the
//! per-shard counters back into one total the same way: field by field,
//! with `saturating_add` so a pathological corpus pins each counter at
//! its maximum instead of wrapping to a small number and understating
//! the work that was actually done.
//!
//! The fold is spelled once here because a hand-written one is a place
//! for a field to be forgotten: a counter that is added in the struct
//! but missed in the fold reports a shard's worth of work as zero, and
//! nothing fails — the number is simply quietly wrong.

/// Folds each named counter of `source` into `target` in place.
macro_rules! absorb_counters {
    ($target:ident, $source:ident, $($field:ident),+ $(,)?) => {
        $($target.$field = $target.$field.saturating_add($source.$field);)+
    };
}

/// Builds `type` from the pairwise sums of each named counter of `left`
/// and `right`. Usable in a `const fn`, unlike the in-place fold.
macro_rules! summed_counters {
    ($type:ident, $left:ident, $right:ident, $($field:ident),+ $(,)?) => {
        $type { $($field: $left.$field.saturating_add($right.$field),)+ }
    };
}

pub(crate) use absorb_counters;
pub(crate) use summed_counters;
