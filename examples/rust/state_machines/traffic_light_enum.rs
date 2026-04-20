//! Traffic-light FSM driven by an enum + match. Paired with
//! `traffic_light_bool.rs` (bit flags) and `traffic_light_table.rs`
//! (transition lookup table) — three different representations of
//! the same state machine.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Light {
    Red,
    Green,
    Yellow,
}

pub fn next(state: Light) -> Light {
    match state {
        Light::Red => Light::Green,
        Light::Green => Light::Yellow,
        Light::Yellow => Light::Red,
    }
}

pub fn run(initial: Light, steps: usize) -> Vec<Light> {
    let mut history: Vec<Light> = Vec::with_capacity(steps.saturating_add(1));
    history.push(initial);
    let mut current = initial;
    for _ in 0..steps {
        current = next(current);
        history.push(current);
    }
    history
}

pub fn ticks_until(initial: Light, target: Light) -> usize {
    if initial == target {
        return 0;
    }
    let mut current = initial;
    for ticks in 1..=3 {
        current = next(current);
        if current == target {
            return ticks;
        }
    }
    0
}
