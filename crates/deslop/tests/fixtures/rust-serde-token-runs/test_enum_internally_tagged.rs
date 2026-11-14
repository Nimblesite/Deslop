    fn tuple() {
        let value = InternallyTagged::NewtypeEnum(Enum::Tuple(1, 1));

        // Special case: tag field ("tag") is the first field
        assert_tokens(
            &value,
            &[
                Token::Map { len: Some(2) },
                Token::Str("tag"),
                Token::Str("NewtypeEnum"),
                Token::Str("Tuple"),
                Token::TupleStruct {
                    name: "Tuple",
                    len: 2,
                },
                Token::U8(1),
                Token::U8(1),
                Token::TupleStructEnd,
                Token::MapEnd,
            ],
        );
        assert_de_tokens(
            &value,
            &[
                Token::Map { len: Some(2) },
                Token::BorrowedStr("tag"),
                Token::BorrowedStr("NewtypeEnum"),
                Token::BorrowedStr("Tuple"),
                Token::TupleStruct {
                    name: "Tuple",
                    len: 2,
                },
                Token::U8(1),
                Token::U8(1),
                Token::TupleStructEnd,
                Token::MapEnd,
            ],
        );

        // Special case: tag field ("tag") is not the first field
        // Reaches crate::private::de::content::VariantDeserializer::tuple_variant
        // Content::Seq case
        // via ContentDeserializer::deserialize_enum
        assert_de_tokens(
            &value,
            &[
                Token::Map { len: Some(2) },
                Token::Str("Tuple"),
                Token::TupleStruct {
                    name: "Tuple",
                    len: 2,
                },
                Token::U8(1),
                Token::U8(1),
                Token::TupleStructEnd,
                Token::Str("tag"),
                Token::Str("NewtypeEnum"),
                Token::MapEnd,
            ],
        );
        assert_de_tokens(
            &value,
            &[
                Token::Map { len: Some(2) },
                Token::BorrowedStr("Tuple"),
                Token::TupleStruct {
                    name: "Tuple",
                    len: 2,
                },
                Token::U8(1),
                Token::U8(1),
                Token::TupleStructEnd,
                Token::BorrowedStr("tag"),
                Token::BorrowedStr("NewtypeEnum"),
                Token::MapEnd,
            ],
        );
    }
    fn struct_() {
        let value = InternallyTagged::NewtypeEnum(Enum::Struct { f: 1 });

        // Special case: tag field ("tag") is the first field
        assert_tokens(
            &value,
            &[
                Token::Map { len: Some(2) },
                Token::Str("tag"),
                Token::Str("NewtypeEnum"),
                Token::Str("Struct"),
                Token::Struct {
                    name: "Struct",
                    len: 1,
                },
                Token::Str("f"),
                Token::U8(1),
                Token::StructEnd,
                Token::MapEnd,
            ],
        );
        assert_de_tokens(
            &value,
            &[
                Token::Map { len: Some(2) },
                Token::BorrowedStr("tag"),
                Token::BorrowedStr("NewtypeEnum"),
                Token::BorrowedStr("Struct"),
                Token::Struct {
                    name: "Struct",
                    len: 1,
                },
                Token::BorrowedStr("f"),
                Token::U8(1),
                Token::StructEnd,
                Token::MapEnd,
            ],
        );

        // General case: tag field ("tag") is not the first field
        // Reaches crate::private::de::content::VariantDeserializer::struct_variant
        // Content::Map case
        // via ContentDeserializer::deserialize_enum
        assert_de_tokens(
            &value,
            &[
                Token::Map { len: Some(2) },
                Token::Str("Struct"),
                Token::Struct {
                    name: "Struct",
                    len: 1,
                },
                Token::Str("f"),
                Token::U8(1),
                Token::StructEnd,
                Token::Str("tag"),
                Token::Str("NewtypeEnum"),
                Token::MapEnd,
            ],
        );
        assert_de_tokens(
            &value,
            &[
                Token::Map { len: Some(2) },
                Token::BorrowedStr("Struct"),
                Token::Struct {
                    name: "Struct",
                    len: 1,
                },
                Token::BorrowedStr("f"),
                Token::U8(1),
                Token::StructEnd,
                Token::BorrowedStr("tag"),
                Token::BorrowedStr("NewtypeEnum"),
                Token::MapEnd,
            ],
        );

        // Special case: tag field ("tag") is the first field
        assert_de_tokens(
            &value,
            &[
                Token::Map { len: Some(2) },
                Token::Str("tag"),
                Token::Str("NewtypeEnum"),
                Token::Str("Struct"),
                Token::Seq { len: Some(1) },
                Token::U8(1), // f
                Token::SeqEnd,
                Token::MapEnd,
            ],
        );
        assert_de_tokens(
            &value,
            &[
                Token::Map { len: Some(2) },
                Token::BorrowedStr("tag"),
                Token::BorrowedStr("NewtypeEnum"),
                Token::BorrowedStr("Struct"),
                Token::Seq { len: Some(1) },
                Token::U8(1), // f
                Token::SeqEnd,
                Token::MapEnd,
            ],
        );

        // General case: tag field ("tag") is not the first field
        // Reaches crate::private::de::content::VariantDeserializer::struct_variant
        // Content::Seq case
        // via ContentDeserializer::deserialize_enum
        assert_de_tokens(
            &value,
            &[
                Token::Map { len: Some(2) },
                Token::Str("Struct"),
                Token::Seq { len: Some(1) },
                Token::U8(1), // f
                Token::SeqEnd,
                Token::Str("tag"),
                Token::Str("NewtypeEnum"),
                Token::MapEnd,
            ],
        );
        assert_de_tokens(
            &value,
            &[
                Token::Map { len: Some(2) },
                Token::BorrowedStr("Struct"),
                Token::Seq { len: Some(1) },
                Token::U8(1), // f
                Token::SeqEnd,
                Token::BorrowedStr("tag"),
                Token::BorrowedStr("NewtypeEnum"),
                Token::MapEnd,
            ],
        );
    }
