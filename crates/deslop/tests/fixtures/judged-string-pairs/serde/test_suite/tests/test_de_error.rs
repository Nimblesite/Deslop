fn test_nonzero_i128() {
    let test = assert_de_tokens_error::<NonZeroI128>;

    // from zero
    test(
        &[Token::I8(0)],
        "invalid value: integer `0`, expected a nonzero i128",
    );
    test(
        &[Token::I16(0)],
        "invalid value: integer `0`, expected a nonzero i128",
    );
    test(
        &[Token::I32(0)],
        "invalid value: integer `0`, expected a nonzero i128",
    );
    test(
        &[Token::I64(0)],
        "invalid value: integer `0`, expected a nonzero i128",
    );
    test(
        &[Token::U8(0)],
        "invalid value: integer `0`, expected a nonzero i128",
    );
    test(
        &[Token::U16(0)],
        "invalid value: integer `0`, expected a nonzero i128",
    );
    test(
        &[Token::U32(0)],
        "invalid value: integer `0`, expected a nonzero i128",
    );
    test(
        &[Token::U64(0)],
        "invalid value: integer `0`, expected a nonzero i128",
    );
}

#[test]
fn test_nonzero_isize() {
    let test = assert_de_tokens_error::<NonZeroIsize>;

    // from zero
    test(
        &[Token::I8(0)],
        "invalid value: integer `0`, expected a nonzero isize",
    );
    test(
        &[Token::I16(0)],
        "invalid value: integer `0`, expected a nonzero isize",
    );
    test(
        &[Token::I32(0)],
        "invalid value: integer `0`, expected a nonzero isize",
    );
    test(
        &[Token::I64(0)],
        "invalid value: integer `0`, expected a nonzero isize",
    );
    test(
        &[Token::U8(0)],
        "invalid value: integer `0`, expected a nonzero isize",
    );
    test(
        &[Token::U16(0)],
        "invalid value: integer `0`, expected a nonzero isize",
    );
    test(
        &[Token::U32(0)],
        "invalid value: integer `0`, expected a nonzero isize",
    );
    test(
        &[Token::U64(0)],
        "invalid value: integer `0`, expected a nonzero isize",
    );
}

#[test]

fn test_nonzero_u64() {
    let test = assert_de_tokens_error::<NonZeroU64>;

    // from zero
    test(
        &[Token::I8(0)],
        "invalid value: integer `0`, expected a nonzero u64",
    );
    test(
        &[Token::I16(0)],
        "invalid value: integer `0`, expected a nonzero u64",
    );
    test(
        &[Token::I32(0)],
        "invalid value: integer `0`, expected a nonzero u64",
    );
    test(
        &[Token::I64(0)],
        "invalid value: integer `0`, expected a nonzero u64",
    );
    test(
        &[Token::U8(0)],
        "invalid value: integer `0`, expected a nonzero u64",
    );
    test(
        &[Token::U16(0)],
        "invalid value: integer `0`, expected a nonzero u64",
    );
    test(
        &[Token::U32(0)],
        "invalid value: integer `0`, expected a nonzero u64",
    );
    test(
        &[Token::U64(0)],
        "invalid value: integer `0`, expected a nonzero u64",
    );

    // from signed
    test(
        &[Token::I8(-1)],
        "invalid value: integer `-1`, expected a nonzero u64",
    );
    test(
        &[Token::I16(-1)],
        "invalid value: integer `-1`, expected a nonzero u64",
    );
    test(
        &[Token::I32(-1)],
        "invalid value: integer `-1`, expected a nonzero u64",
    );
    test(
        &[Token::I64(-1)],
        "invalid value: integer `-1`, expected a nonzero u64",
    );
}

#[test]
fn test_nonzero_u128() {
    let test = assert_de_tokens_error::<NonZeroU128>;

    // from zero
    test(
        &[Token::I8(0)],
        "invalid value: integer `0`, expected a nonzero u128",
    );
    test(
        &[Token::I16(0)],
        "invalid value: integer `0`, expected a nonzero u128",
    );
    test(
        &[Token::I32(0)],
        "invalid value: integer `0`, expected a nonzero u128",
    );
    test(
        &[Token::I64(0)],
        "invalid value: integer `0`, expected a nonzero u128",
    );
    test(
        &[Token::U8(0)],
        "invalid value: integer `0`, expected a nonzero u128",
    );
    test(
        &[Token::U16(0)],
        "invalid value: integer `0`, expected a nonzero u128",
    );
    test(
        &[Token::U32(0)],
        "invalid value: integer `0`, expected a nonzero u128",
    );
    test(
        &[Token::U64(0)],
        "invalid value: integer `0`, expected a nonzero u128",
    );

    // from signed
    test(
        &[Token::I8(-1)],
        "invalid value: integer `-1`, expected a nonzero u128",
    );
    test(
        &[Token::I16(-1)],
        "invalid value: integer `-1`, expected a nonzero u128",
    );
    test(
        &[Token::I32(-1)],
        "invalid value: integer `-1`, expected a nonzero u128",
    );
    test(
        &[Token::I64(-1)],
        "invalid value: integer `-1`, expected a nonzero u128",
    );
}

#[test]
