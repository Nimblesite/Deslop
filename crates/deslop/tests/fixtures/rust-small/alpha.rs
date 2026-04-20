pub fn compute(input: i64) -> i64 {
    if input < 0 {
        return 0;
    }
    let mut total: i64 = 0;
    for index in 0..input {
        total = total + index;
    }
    total
}
