//! [PIPELINE-CLUSTER-SUBSUME-KERNEL] Survivor selection inside one file
//! set.
//!
//! The views over one file set, in rank order, and the same-region
//! preference between every pair that has one, form a directed graph: an
//! edge runs from the view the survivor order prefers to the view it
//! re-describes. The published set is that graph's kernel — no published
//! view is beaten by another published view, and every unpublished view
//! is beaten by a published one. It is found by publishing, in rank
//! order, whichever undecided view no undecided or published view beats,
//! and absorbing the rest of its region as it goes. Publication is a
//! property of the views that end up published, not of the order they
//! were met in, so a view whose absorber leaves the set is judged again
//! against what remains: the release rule of [PIPELINE-CLUSTER-SUBSUME]
//! with nothing forgotten, and no rescan to forget it in.
//!
//! When every undecided view is beaten by another undecided view, the
//! survivor order has a cycle — enclosure decides one pair, and the
//! coverage-mass-id order decides the others the opposite way — and no
//! kernel exists. [PIPELINE-CLUSTER-SUBSUME-CYCLE] publishes the
//! undecided view that leads on the coverage-mass-id order and absorbs
//! the rest of its region.
//!
//! Straddles ([PIPELINE-CLUSTER-SUBSUME-STRADDLE]) are looked for among
//! the published views, removed for good, and the kernel is found again
//! without them.

use std::{cmp::Ordering, collections::BTreeSet};

use super::{
    all_occurrences_overlap, log_subsumption, strictly_encloses,
    survivor::{outranks, same_region_survivor, Preference},
    tally::SubsumeTally,
    Cluster,
};

/// The views over one file set, in rank order, with the same-region
/// preference between every pair that has one.
pub(super) struct Region<'a> {
    /// Views in rank order.
    views: Vec<&'a Cluster>,
    /// `beaters[view]`: views that re-describe `view` and outrank it.
    beaters: Vec<Vec<usize>>,
    /// `beaten[view]`: views that `view` re-describes and outranks.
    beaten: Vec<Vec<usize>>,
}

impl<'a> Region<'a> {
    /// Evaluates every pair of `views` once and records who outranks whom.
    pub(super) fn new(views: Vec<&'a Cluster>, tally: &mut SubsumeTally) -> Self {
        let count = views.len();
        let mut region = Self {
            views,
            beaters: vec![Vec::new(); count],
            beaten: vec![Vec::new(); count],
        };
        for first in 0..count {
            for second in first.saturating_add(1)..count {
                region.connect(first, second, tally);
            }
        }
        region
    }

    /// Records the survivor of `first` and `second`, when they describe
    /// the same duplication.
    fn connect(&mut self, first: usize, second: usize, tally: &mut SubsumeTally) {
        let Some((left, right)) = self.pair(first, second) else {
            return;
        };
        let preference = same_region_survivor(left, right);
        tally.evaluated(preference);
        match preference {
            Preference::First => self.edge(first, second),
            Preference::Second => self.edge(second, first),
            Preference::Neither => {}
        }
    }

    /// Records that `winner` re-describes and outranks `loser`.
    fn edge(&mut self, winner: usize, loser: usize) {
        if let Some(list) = self.beaters.get_mut(loser) {
            list.push(winner);
        }
        if let Some(list) = self.beaten.get_mut(winner) {
            list.push(loser);
        }
    }

    /// How many views the region holds.
    fn len(&self) -> usize {
        self.views.len()
    }

    /// The views that re-describe `view` and outrank it.
    fn beaters(&self, view: usize) -> &[usize] {
        self.beaters.get(view).map_or(&[], Vec::as_slice)
    }

    /// The views that `view` re-describes and outranks.
    fn beaten(&self, view: usize) -> &[usize] {
        self.beaten.get(view).map_or(&[], Vec::as_slice)
    }

    /// Both clusters, when both positions exist.
    fn pair(&self, first: usize, second: usize) -> Option<(&'a Cluster, &'a Cluster)> {
        Some((*self.views.get(first)?, *self.views.get(second)?))
    }

    /// [PIPELINE-CLUSTER-SUBSUME-STRADDLE] Whether `first` and `second`
    /// are two padded readings of a third view of the region: every
    /// occurrence of each overlaps an occurrence of the other in its
    /// file, and some other view lies strictly inside both. File coverage
    /// holds by construction — the region is one file set.
    fn straddle(&self, first: usize, second: usize) -> bool {
        let Some((left, right)) = self.pair(first, second) else {
            return false;
        };
        all_occurrences_overlap(&left.members, &right.members)
            && all_occurrences_overlap(&right.members, &left.members)
            && self.views.iter().enumerate().any(|(index, core)| {
                index != first
                    && index != second
                    && strictly_encloses(&left.members, &core.members)
                    && strictly_encloses(&right.members, &core.members)
            })
    }
}

/// Resolves one region: the positions of its published views, in rank
/// order.
pub(super) fn resolve(region: &Region<'_>, tally: &mut SubsumeTally) -> Vec<usize> {
    let mut straddled = vec![false; region.len()];
    loop {
        let published = kernel(region, &straddled, tally);
        let pairs = straddling_pairs(region, &published);
        if pairs.is_empty() {
            return published;
        }
        tally.straddle_round(pairs.len());
        for (first, second) in pairs {
            remove_straddlers(region, &mut straddled, first, second);
        }
    }
}

/// Every pair of published views that straddles a third view, in rank
/// order.
fn straddling_pairs(region: &Region<'_>, published: &[usize]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for (position, first) in published.iter().enumerate() {
        let later = published
            .get(position.saturating_add(1)..)
            .unwrap_or_default();
        for second in later {
            if region.straddle(*first, *second) {
                pairs.push((*first, *second));
            }
        }
    }
    pairs
}

/// Removes both straddlers for good; the nested view is the finding.
fn remove_straddlers(region: &Region<'_>, straddled: &mut [bool], first: usize, second: usize) {
    if let Some((left, right)) = region.pair(first, second) {
        log_subsumption(right, left, "drop_both_straddle");
    }
    for index in [first, second] {
        if let Some(slot) = straddled.get_mut(index) {
            *slot = true;
        }
    }
}

/// Publication state of one view while its region is resolved.
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    /// Not yet judged against the published set.
    Undecided,
    /// A finding.
    Published,
    /// Re-described and outranked by a published view.
    Absorbed,
    /// Removed as a straddler in an earlier round.
    Straddled,
}

/// Live bookkeeping for one kernel computation over a region.
struct Resolution<'r, 'a> {
    /// The region being resolved.
    region: &'r Region<'a>,
    /// Each view's state.
    states: Vec<State>,
    /// How many undecided or published views still beat each view.
    live_beaters: Vec<usize>,
    /// Undecided views nothing live beats, in rank order.
    ready: BTreeSet<usize>,
}

/// The kernel of the region with `straddled` views removed: published
/// positions in rank order.
fn kernel(region: &Region<'_>, straddled: &[bool], tally: &mut SubsumeTally) -> Vec<usize> {
    let mut resolution = Resolution::new(region, straddled);
    let mut published = Vec::new();
    while let Some(next) = resolution
        .next_source()
        .or_else(|| resolution.break_cycle(tally))
    {
        resolution.publish(next, tally);
        published.push(next);
    }
    published.sort_unstable();
    published
}

impl<'r, 'a> Resolution<'r, 'a> {
    /// Every view undecided except the straddlers, with its live-beater
    /// count and the initial ready set.
    fn new(region: &'r Region<'a>, straddled: &[bool]) -> Self {
        let gone = |view: usize| straddled.get(view).copied().unwrap_or(true);
        let states: Vec<State> = (0..region.len())
            .map(|view| if gone(view) { State::Straddled } else { State::Undecided })
            .collect();
        let live_beaters: Vec<usize> = (0..region.len())
            .map(|view| region.beaters(view).iter().filter(|beater| !gone(**beater)).count())
            .collect();
        let ready = states
            .iter()
            .zip(&live_beaters)
            .enumerate()
            .filter(|(_, (state, live))| **state == State::Undecided && **live == 0)
            .map(|(view, _)| view)
            .collect();
        Self { region, states, live_beaters, ready }
    }

    /// The best-ranked undecided view nothing live beats.
    fn next_source(&mut self) -> Option<usize> {
        self.ready.pop_first()
    }

    /// [PIPELINE-CLUSTER-SUBSUME-CYCLE] With every undecided view beaten
    /// by another, the view that leads on occurrence coverage, mass and
    /// id is the finding.
    fn break_cycle(&self, tally: &mut SubsumeTally) -> Option<usize> {
        let best = (0..self.region.len())
            .filter(|view| self.state(*view) == State::Undecided)
            .max_by(|left, right| {
                self.region
                    .pair(*left, *right)
                    .map_or(Ordering::Equal, |(first, second)| outranks(first, second))
            })?;
        tally.cycle_broken();
        Some(best)
    }

    /// Publishes `view` and absorbs every undecided view of its region.
    fn publish(&mut self, view: usize, tally: &mut SubsumeTally) {
        self.set(view, State::Published);
        let region = self.region;
        let rivals = region.beaten(view).iter().chain(region.beaters(view));
        for rival in rivals {
            if self.state(*rival) == State::Undecided {
                self.absorb(*rival, view, tally);
            }
        }
    }

    /// Absorbs `rival` into `survivor`, freeing the views `rival` beat.
    fn absorb(&mut self, rival: usize, survivor: usize, tally: &mut SubsumeTally) {
        self.set(rival, State::Absorbed);
        tally.absorbed();
        let _ = self.ready.remove(&rival);
        let region = self.region;
        if let Some((winner, loser)) = region.pair(survivor, rival) {
            log_subsumption(winner, loser, "absorbed");
        }
        for freed in region.beaten(rival) {
            self.release_beater(*freed);
        }
    }

    /// One fewer live view beats `view`; with none left it is ready.
    fn release_beater(&mut self, view: usize) {
        let Some(count) = self.live_beaters.get_mut(view) else {
            return;
        };
        *count = count.saturating_sub(1);
        if *count == 0 && self.state(view) == State::Undecided {
            let _ = self.ready.insert(view);
        }
    }

    /// The state of `view`; positions outside the region count as gone.
    fn state(&self, view: usize) -> State {
        self.states.get(view).copied().unwrap_or(State::Straddled)
    }

    /// Sets the state of `view` when the position exists.
    fn set(&mut self, view: usize, state: State) {
        if let Some(slot) = self.states.get_mut(view) {
            *slot = state;
        }
    }
}
