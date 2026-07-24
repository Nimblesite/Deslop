pub fn shift(state: usize) -> usize {
    match state {
        0 => 2,
        value if value > 10 => 0,
        value => value * 3,
    }
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

pub fn report_b(values: &[usize]) -> String {
    values
        .iter()
        .map(|value| format!("b:{value}"))
        .collect::<Vec<String>>()
        .join(",")
}
