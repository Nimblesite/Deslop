pub fn shift(state: usize) -> usize {
    state + 1
}

pub fn run(initial: usize, steps: usize) -> Vec<usize> {
    let mut history: Vec<usize> = Vec::with_capacity(steps + 1);
    history.push(initial);
    let mut current = initial;
    let mut index = 0;
    while index < steps {
        current = shift(current);
        history.push(current);
        index += 1;
    }
    history
}

pub fn report_a(initial: usize) -> usize {
    run(initial, 3).len()
}
