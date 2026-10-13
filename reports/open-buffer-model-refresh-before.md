# Open-buffer model refresh check

Command: `cargo test -p deslop-lsp --test suite open_buffers::embedding::selected_model_preserves_unsaved_buffer_report -- --exact --nocapture`

Exit status: **101**

Elapsed seconds: **1.845**

Full log: `/home/christian/code/deslop/target/resource-fixes-integration/target/open-buffer-model-refresh-before.log`

```text


thread 'open_buffers::embedding::selected_model_preserves_unsaved_buffer_report' (1201897) panicked at crates/deslop-lsp/tests/open_buffers/embedding.rs:80:5:
the unsaved empty buffer must remain authoritative after model selection: {"boilerplate_hints":[],"cache_stats":{"hits":2,"misses":0},"clusters":[{"canonical_node_count":58,"id":"54a6b248b64f425d","kind":"nearly_identical","mass":58,"occurrence_count":2,"occurrences":[{"end_byte":383,"end_line":19,"hidden":false,"path":"Alpha.cs","start_byte":0,"start_line":1},{"end_byte":414,"end_line":19,"hidden":false,"path":"Beta.cs","start_byte":0,"start_line":1}],"occurrences_total":2,"occurrences_truncated":false,"rank":1,"severity":"warning"}],"clusters_hidden":0,"embedding_provenance":{"attempted_subtrees":34,"dimensions":128,"failed_subtrees":0,"indexed_subtrees":30,"model_id":"nomic-embed-text","model_version":"0123456789ab","provider_id":"ollama","succeeded_subtrees":34},"files_analysed":2,"literal_findings":[],"literal_findings_capped":false,"literal_findings_hidden":0,"literal_findings_total":0,"literal_max_findings":0,"metrics":{"analysed_loc":38,"clusters_total":1,"duplicated_files":2,"duplicated_loc":38,"duplication_percent":100.0,"folders":[],"per_file":[{"analysed_loc":19,"duplicated_loc":19,"duplication_percent":100.0,"path":"Alpha.cs"},{"analysed_loc":19,"duplicated_loc":19,"duplication_percent":100.0,"path":"Beta.cs"}],"threshold":{"breached":false,"percent":0.0,"source":"none"}},"min_nodes":30,"routing":{"nearly_identical_min_content":0.7,"nearly_identical_min_shape":0.9,"shape_only_max_content":0.05,"similar_min_content":0.5},"schema_doc":"","tool_version":"0.0.0-dev"}
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test open_buffers::embedding::selected_model_preserves_unsaved_buffer_report ... FAILED

failures:

failures:
    open_buffers::embedding::selected_model_preserves_unsaved_buffer_report

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 85 filtered out; finished in 0.43s

error: test failed, to rerun pass `-p deslop-lsp --test suite`

```
