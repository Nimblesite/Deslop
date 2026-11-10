// [TESTING-CLI-RUN-SCRATCH] `--run` on a test file keeps relative output out of
// the caller's directory, in both flavors; an ordinary program still writes there.
#[test]
fn run_keeps_test_file_output_out_of_the_callers_directory() {
    const OUTPUT: &str = "relative_output.txt";
    let osp = "let saved = writeFile(\"relative_output.txt\", \"kept\")\n\
               match saved {\n    Success => print(\"saved\")\n    Error => print(\"failed\")\n}\n\
               match readFile(\"relative_output.txt\") {\n    Success => print(\"read\")\n    Error => print(\"lost\")\n}\n";
    let ospml = "saved = writeFile \"relative_output.txt\" \"kept\"\n\
                 match saved\n    Success _value => print \"saved\"\n    Error _message => print \"failed\"\n\
                 match readFile \"relative_output.txt\"\n    Success _value => print \"read\"\n    Error _message => print \"lost\"\n";
    assert_ne!(osp, ospml);
}
