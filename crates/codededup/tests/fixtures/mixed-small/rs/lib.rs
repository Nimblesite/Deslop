pub fn alpha(value: i64) -> i64 {
    if value < 0 {
        return 0;
    }
    let mut total: i64 = 0;
    for index in 0..value {
        total = total + index;
    }
    total
}

pub fn beta(bound: i64) -> i64 {
    if bound < 0 {
        return 0;
    }
    let mut running: i64 = 0;
    for step in 0..bound {
        running = running + step;
    }
    running
}
