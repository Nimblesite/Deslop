    fn enum_tuple_and_struct() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        enum Outer {
            Tuple(f64, i32),
            Flatten {
                #[serde(flatten)]
                nested: Nested,
            },
        }

        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Nested {
            a: i32,
            b: i32,
        }

        assert_tokens(
            &Outer::Tuple(1.2, 3),
            &[
                Token::TupleVariant {
                    name: "Outer",
                    variant: "Tuple",
                    len: 2,
                },
                Token::F64(1.2),
                Token::I32(3),
                Token::TupleVariantEnd,
            ],
        );
        assert_tokens(
            &Outer::Flatten {
                nested: Nested { a: 1, b: 2 },
            },
            &[
                Token::NewtypeVariant {
                    name: "Outer",
                    variant: "Flatten",
                },
                Token::Map { len: None },
                Token::Str("a"),
                Token::I32(1),
                Token::Str("b"),
                Token::I32(2),
                Token::MapEnd,
            ],
        );
    }
            fn straightforward() {
                #[derive(Serialize, Deserialize, PartialEq, Debug)]
                enum Data {
                    A {
                        a: i32,
                        #[serde(flatten)]
                        flat: Flat,
                    },
                }

                #[derive(Serialize, Deserialize, PartialEq, Debug)]
                struct Flat {
                    b: i32,
                }

                let data = Data::A {
                    a: 0,
                    flat: Flat { b: 0 },
                };

                assert_tokens(
                    &data,
                    &[
                        Token::NewtypeVariant {
                            name: "Data",
                            variant: "A",
                        },
                        Token::Map { len: None },
                        Token::Str("a"),
                        Token::I32(0),
                        Token::Str("b"),
                        Token::I32(0),
                        Token::MapEnd,
                    ],
                );
            }
            fn tuple() {
                let value = Flatten {
                    data: Enum::Tuple(0, 42),
                    extra: HashMap::from_iter([("extra_key".into(), "extra value".into())]),
                };
                assert_tokens(
                    &value,
                    &[
                        Token::Map { len: None },
                        // data
                        Token::Str("Tuple"), // variant
                        Token::Seq { len: Some(2) },
                        Token::U32(0),
                        Token::U32(42),
                        Token::SeqEnd,
                        // extra
                        Token::Str("extra_key"),
                        Token::Str("extra value"),
                        Token::MapEnd,
                    ],
                );
                assert_de_tokens(
                    &value,
                    &[
                        Token::Map { len: None },
                        // extra
                        Token::Str("extra_key"),
                        Token::Str("extra value"),
                        // data
                        Token::Str("Tuple"), // variant
                        Token::Seq { len: Some(2) },
                        Token::U32(0),
                        Token::U32(42),
                        Token::SeqEnd,
                        Token::MapEnd,
                    ],
                );
            }
            fn struct_from_seq() {
                let value = Flatten {
                    data: Enum::Struct {
                        index: 0,
                        value: 42,
                    },
                    extra: HashMap::from_iter([("extra_key".into(), "extra value".into())]),
                };
                assert_de_tokens(
                    &value,
                    &[
                        Token::Map { len: None },
                        // data
                        Token::Str("Struct"), // variant
                        Token::Seq { len: Some(2) },
                        Token::U32(0),  // index
                        Token::U32(42), // value
                        Token::SeqEnd,
                        // extra
                        Token::Str("extra_key"),
                        Token::Str("extra value"),
                        Token::MapEnd,
                    ],
                );
                assert_de_tokens(
                    &value,
                    &[
                        Token::Map { len: None },
                        // extra
                        Token::Str("extra_key"),
                        Token::Str("extra value"),
                        // data
                        Token::Str("Struct"), // variant
                        Token::Seq { len: Some(2) },
                        Token::U32(0),  // index
                        Token::U32(42), // value
                        Token::SeqEnd,
                        Token::MapEnd,
                    ],
                );
            }

            // Reaches crate::private::de::content::VariantDeserializer::struct_variant
