pub fn legacy_checksum(items: &[u32]) -> u32 {
    let mut state = 7_u32;
    for item in items {
        state = state.wrapping_mul(31).wrapping_add(*item);
        if state % 5 == 0 {
            state = state.rotate_left(3);
        }
    }
    state ^ 0x5a5a
}
