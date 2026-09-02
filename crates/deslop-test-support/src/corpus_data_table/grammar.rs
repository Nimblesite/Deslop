//! [CORPUS-PRECISION] Per-language table grammar (gh #452).
//!
//! Curated from the parsed s-expression of a three-entry table in each
//! corpus language, not from memory. One rule holds across all nine
//! grammars and is why no per-language value field is needed: **a
//! declaration's value slot is its last named child** — `assignment`
//! (left, right), `const_item` (name, type, value), `variable_declarator`
//! (name, value), `const_spec` (name, value), `static_final_declaration`
//! (name, value), `const_element` (name, value), `function_or_value_defn`
//! (left, body).

/// The node kinds that make a table in one language.
pub(super) struct TableGrammar {
    /// Declaration kinds whose value slot holds one table entry.
    pub(super) declarations: &'static [&'static str],
    /// Collection kinds whose named children are table entries.
    pub(super) collections: &'static [&'static str],
    /// Literal kinds — the only thing a table entry may be. Reaching one
    /// stops the unwrap, so a string's internal `string_content` nodes are
    /// never inspected and its *text* never influences the verdict.
    pub(super) literals: &'static [&'static str],
    /// Kinds that make a span logic whatever else it holds. A call is the
    /// discriminator: a config object or a test setup block holds plenty
    /// of literal arrays, but it *calls* something, so it is code. Erring
    /// this way costs the gate a data table built from constructor calls
    /// — the shape [CLONE-NOISE-DART-DATA-TABLE-LITERAL] handles — and a
    /// missed gate failure is far cheaper than a false one.
    pub(super) logic: &'static [&'static str],
}

/// Every corpus language, keyed by the engine's parser id. A language
/// absent here makes the check error rather than pass without judging
/// anything, the stance gh #401 took for the heritage grammar.
pub(super) const TABLE_GRAMMARS: &[(&str, TableGrammar)] = &[
    (
        "python",
        TableGrammar {
            declarations: &["assignment"],
            collections: &["list", "set", "dictionary", "tuple"],
            literals: &["string", "integer", "float", "true", "false", "none"],
            logic: &["call"],
        },
    ),
    (
        "rust",
        TableGrammar {
            declarations: &["const_item", "static_item"],
            collections: &["array_expression", "tuple_expression"],
            literals: &[
                "string_literal",
                "raw_string_literal",
                "integer_literal",
                "float_literal",
                "boolean_literal",
                "char_literal",
            ],
            logic: &["call_expression", "macro_invocation"],
        },
    ),
    (
        "typescript",
        TableGrammar {
            declarations: &["variable_declarator", "public_field_definition"],
            collections: &["array", "object"],
            literals: &[
                "string",
                "template_string",
                "number",
                "true",
                "false",
                "null",
            ],
            logic: &["call_expression", "new_expression"],
        },
    ),
    (
        "javascript",
        TableGrammar {
            declarations: &["variable_declarator", "field_definition"],
            collections: &["array", "object"],
            literals: &[
                "string",
                "template_string",
                "number",
                "true",
                "false",
                "null",
            ],
            logic: &["call_expression", "new_expression"],
        },
    ),
    (
        "csharp",
        TableGrammar {
            declarations: &["variable_declarator"],
            collections: &["initializer_expression", "collection_expression"],
            literals: &[
                "string_literal",
                "verbatim_string_literal",
                "raw_string_literal",
                "integer_literal",
                "real_literal",
                "boolean_literal",
                "character_literal",
                "null_literal",
            ],
            logic: &["invocation_expression", "object_creation_expression"],
        },
    ),
    (
        "go",
        TableGrammar {
            declarations: &["const_spec", "var_spec"],
            collections: &["literal_value"],
            literals: &[
                "interpreted_string_literal",
                "raw_string_literal",
                "int_literal",
                "float_literal",
                "imaginary_literal",
                "rune_literal",
            ],
            logic: &["call_expression"],
        },
    ),
    (
        "dart",
        TableGrammar {
            declarations: &[
                "static_final_declaration",
                "initialized_variable_definition",
            ],
            collections: &["list_literal", "set_or_map_literal"],
            literals: &[
                "string_literal",
                "decimal_integer_literal",
                "decimal_floating_point_literal",
                "hex_integer_literal",
                "true",
                "false",
            ],
            logic: &["call_expression"],
        },
    ),
    (
        "php",
        TableGrammar {
            declarations: &["const_element", "assignment_expression"],
            collections: &["array_creation_expression"],
            literals: &["encapsed_string", "string", "integer", "float", "boolean"],
            logic: &[
                "function_call_expression",
                "member_call_expression",
                "object_creation_expression",
                "scoped_call_expression",
            ],
        },
    ),
    (
        "fsharp",
        TableGrammar {
            declarations: &["function_or_value_defn"],
            collections: &["list_expression", "array_expression"],
            literals: &["const"],
            logic: &["application_expression"],
        },
    ),
];
