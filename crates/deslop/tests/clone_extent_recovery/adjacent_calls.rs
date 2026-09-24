//! [PIPELINE-CLUSTER-EXACT-ADJACENT] A byte-identical run of TypeScript calls stays whole.

use std::{fs, path::Path};

use super::*;

const LEFT_FILE: &str = "Cjs.ts";
const RIGHT_FILE: &str = "Esm.ts";
const LEFT_PREFIX: &str = "interface LeftTag { cjs: string }\n";
const RIGHT_PREFIX: &str = "function rightTag() { return true; }\n";
const LEFT_SUFFIX: &str = "export const leftEnd = 1;\n";
const RIGHT_SUFFIX: &str = "class RightEnd {}\n";
const SHARED_START: u64 = 2;
const SHARED_END: u64 = 32;
const SHARED_LINES: usize = 31;
const ANALYSED_FILES: u64 = 2;
const MIN_NODES: u32 = 30;
const FIRST_RANK: u64 = 1;
const COPIES: u64 = 2;

const SHARED_CALLS: &str = "const handleStringResponse = (response: string) => {\n\
  console.log(response);\n\
};\n\
\n\
axios.get<User, string>('/user?id=12345')\n\
    .then(handleStringResponse)\n\
    .catch(handleError);\n\
\n\
axios.get<User, string>('/user', { params: { id: 12345 } })\n\
    .then(handleStringResponse)\n\
    .catch(handleError);\n\
\n\
axios.head<User, string>('/user')\n\
    .then(handleStringResponse)\n\
    .catch(handleError);\n\
\n\
axios.options<User, string>('/user')\n\
    .then(handleStringResponse)\n\
    .catch(handleError);\n\
\n\
axios.delete<User, string>('/user')\n\
    .then(handleStringResponse)\n\
    .catch(handleError);\n\
\n\
axios.post<Partial<UserCreationDef>, string>('/user', { name: 'foo' })\n\
    .then(handleStringResponse)\n\
    .catch(handleError);\n\
\n\
axios.post<Partial<UserCreationDef>, string>('/user', { name: 'foo' }, { headers: { 'X-FOO': 'bar' } })\n\
    .then(handleStringResponse)\n\
    .catch(handleError);\n";

fn write_fixture(root: &Path) -> Result<()> {
    assert_eq!(SHARED_CALLS.lines().count(), SHARED_LINES);
    fs::write(root.join(LEFT_FILE), format!("{LEFT_PREFIX}{SHARED_CALLS}{LEFT_SUFFIX}"))?;
    fs::write(root.join(RIGHT_FILE), format!("{RIGHT_PREFIX}{SHARED_CALLS}{RIGHT_SUFFIX}"))?;
    Ok(())
}

fn has_full_side(cluster: &Value, file: &str) -> bool {
    occurrences(cluster).iter().any(|occurrence| {
        occurrence_path(occurrence).is_ok_and(|path| path.ends_with(file))
            && occurrence_line_span(occurrence) == (SHARED_START, SHARED_END)
    })
}

#[test]
fn identical_adjacent_calls_keep_their_full_extent() -> Result<()> {
    let (_tmp, root) = temp_scan_dir("src")?;
    write_fixture(&root)?;
    let report = run_report(&root, MIN_NODES)?;
    let exact: Vec<_> = clone_findings(&report)
        .into_iter()
        .filter(|cluster| {
            cluster_kind(cluster) == IDENTICAL_KIND
                && [LEFT_FILE, RIGHT_FILE]
                    .iter()
                    .all(|file| has_full_side(cluster, file))
        })
        .collect();
    assert_eq!(field(&report, "files_analysed").as_u64(), Some(ANALYSED_FILES));
    assert_eq!(exact.len(), 1, "complete call run missing: {report:#}");
    let cluster = exact.first().expect("one full exact call-run cluster");
    assert_eq!(cluster_size(cluster), COPIES);
    assert_eq!(field(cluster, "rank").as_u64(), Some(FIRST_RANK));
    assert!(crate::common::signals::has_verbatim_pair(&root, cluster)?);
    Ok(())
}
