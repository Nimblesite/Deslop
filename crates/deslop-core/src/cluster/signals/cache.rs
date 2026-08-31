//! Pair-signal values retained only when a compact group pair repeats.

use std::collections::HashMap;

/// Number of distinct values in the one-group fast path.
const SINGLE_GROUP_COUNT: usize = 1;

/// Compact pair key for that sole group.
const SINGLE_GROUP_PAIR: (usize, usize) = (0, 0);

/// Members required for two distinct same-group occurrence pairs.
const SAME_GROUP_CACHE_MIN_MEMBERS: usize = 3;

/// Members required for a cross-group pair value to repeat.
const CROSS_GROUP_CACHE_MIN_MEMBERS: usize = 2;

/// Values retained only when a group pair occurs more than once.
pub(super) struct PairValueCache {
    /// Population per compact group id.
    group_sizes: Vec<usize>,
    /// Value for the overwhelmingly common one-group cluster, avoiding
    /// a hash-table lookup for every logical pair.
    single_value: Option<f64>,
    /// Already measured repeated group pairs.
    values: HashMap<(usize, usize), f64>,
}

impl PairValueCache {
    /// Cache over one signal's group populations.
    pub(super) fn new(group_sizes: Vec<usize>) -> Self {
        Self {
            group_sizes,
            single_value: None,
            values: HashMap::new(),
        }
    }

    /// Returns one value, retaining it only when another pair reuses it.
    pub(super) fn value(&mut self, groups: (usize, usize), compute: impl FnOnce() -> f64) -> f64 {
        let key = ordered_group_pair(groups);
        if self.group_sizes.len() == SINGLE_GROUP_COUNT && key == SINGLE_GROUP_PAIR {
            return self.single(compute);
        }
        if !self.repeats(key) {
            return compute();
        }
        if let Some(&cached) = self.values.get(&key) {
            return cached;
        }
        let value = compute();
        let _previous = self.values.insert(key, value);
        value
    }

    /// Returns or initializes the sole group's cached value.
    fn single(&mut self, compute: impl FnOnce() -> f64) -> f64 {
        if let Some(cached) = self.single_value {
            return cached;
        }
        let value = compute();
        self.single_value = Some(value);
        value
    }

    /// Whether more than one occurrence pair shares this group pair.
    fn repeats(&self, (left, right): (usize, usize)) -> bool {
        let left_size = self.group_sizes.get(left).copied().unwrap_or(0);
        let right_size = self.group_sizes.get(right).copied().unwrap_or(0);
        if left == right {
            return left_size >= SAME_GROUP_CACHE_MIN_MEMBERS;
        }
        left_size >= CROSS_GROUP_CACHE_MIN_MEMBERS || right_size >= CROSS_GROUP_CACHE_MIN_MEMBERS
    }
}

/// Order-insensitive cache identity for two compact groups.
fn ordered_group_pair((left, right): (usize, usize)) -> (usize, usize) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}
