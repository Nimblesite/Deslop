pub fn run(limit: i64) -> i64 {
    if limit < 0 {
        return 0;
    }
    let mut accumulator: i64 = 0;
    for position in 0..limit {
        accumulator = accumulator + position;
    }
    accumulator
}
