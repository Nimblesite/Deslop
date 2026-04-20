//! Transition-table twin of `traffic_light_enum.rs`. States are `u8`
//! indices; transitions live in a const array. Classic lookup-driven
//! FSM — AST and tokens differ from the enum + match variant, but the
//! embedding signal should still cluster them.

pub type Light = u8;

pub const RED: Light = 0;
pub const GREEN: Light = 1;
pub const YELLOW: Light = 2;

const TRANSITIONS: [Light; 3] = [GREEN, YELLOW, RED];

pub fn next(state: Light) -> Light {
    TRANSITIONS
        .get(state as usize)
        .copied()
        .unwrap_or(RED)
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
