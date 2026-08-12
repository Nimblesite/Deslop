pub fn accumulate(values: &[i64], floor: i64) -> i64 {
    let mut total = 0;
    for value in values {
        if *value > floor {
            total = total + value * 2;
        } else {
            total = total - 1;
        }
    }
    total
}
