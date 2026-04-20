//! Bit-flag twin of `traffic_light_enum.rs`. The state is encoded as
//! a pair of bools `(red_on, green_on)`; yellow is `(false, false)`.
//! Completely different AST and tokens — Type-4 via embeddings.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Light {
    pub red_on: bool,
    pub green_on: bool,
}

pub const RED: Light = Light { red_on: true, green_on: false };
pub const GREEN: Light = Light { red_on: false, green_on: true };
pub const YELLOW: Light = Light { red_on: false, green_on: false };

pub fn next(state: Light) -> Light {
    if state.red_on {
        GREEN
    } else if state.green_on {
        YELLOW
    } else {
        RED
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
