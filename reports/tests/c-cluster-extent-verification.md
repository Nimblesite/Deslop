# C cluster-extent verification

Captured test output; this failure is outside the eight historical regression-report checks.

Revision: `3087198eaffbafa3275426502fe692026c83edab`

Command: `cargo test -p deslop --test suite c_cluster_occurrences_never_pair_a_whole_file_with_a_function_run`

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.24s
     Running tests/suite.rs (target/debug/deps/suite-835e6c1e7ae0008b)

running 1 test
test cli::detection::c::c_cluster_occurrences_never_pair_a_whole_file_with_a_function_run ... FAILED

failures:

---- cli::detection::c::c_cluster_occurrences_never_pair_a_whole_file_with_a_function_run stdout ----

thread 'cli::detection::c::c_cluster_occurrences_never_pair_a_whole_file_with_a_function_run' (6082060) panicked at crates/deslop/tests/cli/detection/c.rs:262:5:
the copied assert helpers pair from row 27: [
    Object {
        "canonical_node_count": Number(408),
        "id": String("46501ee46141025d"),
        "kind": String("loosely_similar"),
        "mass": Number(408),
        "occurrence_count": Number(2),
        "occurrences": Array [
            Object {
                "end_byte": Number(3817),
                "end_line": Number(100),
                "hidden": Bool(false),
                "language": String("c"),
                "path": String("print_array.c"),
                "start_byte": Number(1122),
                "start_line": Number(23),
            },
            Object {
                "end_byte": Number(3436),
                "end_line": Number(83),
                "hidden": Bool(false),
                "language": String("c"),
                "path": String("print_object.c"),
                "start_byte": Number(1213),
                "start_line": Number(27),
            },
        ],
        "occurrences_total": Number(2),
        "occurrences_truncated": Bool(false),
        "rank": Number(1),
        "severity": String("information"),
    },
    Object {
        "canonical_node_count": Number(31),
        "id": String("e6473eec6f75faaf"),
        "kind": String("nearly_identical"),
        "mass": Number(31),
        "occurrence_count": Number(2),
        "occurrences": Array [
            Object {
                "end_byte": Number(3817),
                "end_line": Number(100),
                "hidden": Bool(false),
                "language": String("c"),
                "path": String("print_array.c"),
                "start_byte": Number(3523),
                "start_line": Number(90),
            },
            Object {
                "end_byte": Number(4222),
                "end_line": Number(101),
                "hidden": Bool(false),
                "language": String("c"),
                "path": String("print_object.c"),
                "start_byte": Number(3922),
                "start_line": Number(91),
            },
        ],
        "occurrences_total": Number(2),
        "occurrences_truncated": Bool(false),
        "rank": Number(2),
        "severity": String("warning"),
    },
]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    cli::detection::c::c_cluster_occurrences_never_pair_a_whole_file_with_a_function_run

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 619 filtered out; finished in 2.53s

error: test failed, to rerun pass `-p deslop --test suite`

```
