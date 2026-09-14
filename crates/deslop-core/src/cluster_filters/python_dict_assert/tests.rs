//! Unit pins for [CLONE-NOISE-PY-DICT-ASSERT] at the `is_noise_pattern`
//! seam: the chained-subscript idiom must be convicted at every
//! granularity the reported range intersects — the enclosing `test_*`
//! function view and the assert-run window whose payload sits above it.

use super::super::{is_noise_pattern, NoiseFilter, ParseCache};
use crate::ast::ByteRange;
use crate::fingerprint::Fingerprint;
use crate::state::{FileId, FileRegistry};

use std::collections::HashMap;

/// The pytest module the #107 fixture stages: a payload binding and the
/// two chained assertions that consume it, wrapped in one `test_`
/// function.
const PATCH_MODULE: &str = "def test_configs_patch_model_config_nesting():\n    data = {\"model_config\": {\"provider\": \"openai\", \"model\": \"gpt-4o\"}}\n    assert data[\"model_config\"][\"provider\"] == \"openai\"\n    assert data[\"model_config\"][\"model\"] == \"gpt-4o\"\n";

/// The second unrelated pytest module: same skeleton, different
/// contract, different literal values.
const OPENAPI_MODULE: &str = "def test_openapi_info_document():\n    doc = {\"info\": {\"title\": \"Agent Backend\", \"version\": \"0.1.0\"}}\n    assert doc[\"info\"][\"title\"] == \"Agent Backend\"\n    assert doc[\"info\"][\"version\"] == \"0.1.0\"\n";

/// Byte offset of the `def` line in either module: the function view.
const FUNCTION_START: usize = 0;

/// Byte offset of the first `assert` line in `PATCH_MODULE`.
const PATCH_ASSERT_START: usize = 82;

/// Byte offset of the first `assert` line in `OPENAPI_MODULE`.
const OPENAPI_ASSERT_START: usize = 79;

/// Byte length of one chained assertion line, newline excluded.
const ASSERT_LINE_LEN: usize = 51;

/// Convicting the function view must report the language-specific bank:
/// the chained-dict proof closes over both members.
#[test]
fn chained_dict_assert_function_level_pair_is_convicted() {
    let verdict = Corpus::new()
        .member(PATCH_MODULE, FUNCTION_START, PATCH_MODULE.len() - 1)
        .member(OPENAPI_MODULE, FUNCTION_START, OPENAPI_MODULE.len() - 1)
        .verdict();
    assert_eq!(
        verdict,
        Some(NoiseFilter::LanguageSpecific),
        "a cross-file pair of test functions whose bodies are payload \
         bindings plus chained assertions is the [CLONE-NOISE-PY-DICT-ASSERT] \
         idiom and must be convicted: {verdict:?}"
    );
}

/// The assert-run window qualifies on assertion shape alone: its payload
/// binding sits above the range, the enclosing `test_*` function is what
/// the range intersects, and the proof still closes.
#[test]
fn chained_dict_assert_run_windows_are_convicted() {
    let verdict = Corpus::new()
        .member(
            PATCH_MODULE,
            PATCH_ASSERT_START,
            PATCH_ASSERT_START + ASSERT_LINE_LEN,
        )
        .member(
            OPENAPI_MODULE,
            OPENAPI_ASSERT_START,
            OPENAPI_ASSERT_START + ASSERT_LINE_LEN,
        )
        .verdict();
    assert_eq!(
        verdict,
        Some(NoiseFilter::LanguageSpecific),
        "cross-file assert-run windows over chained subscript lookups are \
         the idiom at statement granularity and must be convicted: {verdict:?}"
    );
}

/// Two members of one file never meet the two-file suppression clause —
/// the filter must decline and leave the pair to the admission contract.
#[test]
fn single_file_assert_pair_is_not_convicted() {
    let verdict = Corpus::new()
        .member(
            PATCH_MODULE,
            PATCH_ASSERT_START,
            PATCH_ASSERT_START + ASSERT_LINE_LEN,
        )
        .member(
            PATCH_MODULE,
            PATCH_ASSERT_START,
            PATCH_ASSERT_START + ASSERT_LINE_LEN,
        )
        .verdict();
    assert_eq!(
        verdict, None,
        "the same-file pair does not meet the two-file clause of \
         [CLONE-NOISE-PY-DICT-ASSERT] and must not be convicted here: {verdict:?}"
    );
}

/// The member corpus: registered sources plus one fingerprint per
/// member, mirroring the whole-filter harness the polymorphic pins use.
struct Corpus {
    registry: FileRegistry,
    sources: Vec<(FileId, &'static str)>,
    members: Vec<Fingerprint>,
}

impl Corpus {
    fn new() -> Self {
        Self {
            registry: FileRegistry::new(),
            sources: Vec::new(),
            members: Vec::new(),
        }
    }

    /// Registers `source` and offers the byte range as one member.
    fn member(mut self, source: &'static str, start: usize, end: usize) -> Self {
        let file_id = self.registry.register(std::path::PathBuf::from("src.py"));
        self.sources.push((file_id, source));
        self.members.push(Fingerprint {
            hash: [0_u8; 32],
            file_id,
            byte_range: ByteRange {
                start,
                end: end.min(source.len()),
            },
            node_count: 20,
        });
        self
    }

    /// Runs the whole noise bank over the members.
    fn verdict(self) -> Option<NoiseFilter> {
        let sources: HashMap<FileId, Vec<u8>> = self
            .sources
            .iter()
            .map(|(file_id, text)| (*file_id, text.as_bytes().to_vec()))
            .collect();
        let languages: HashMap<FileId, &'static str> = self
            .sources
            .iter()
            .map(|(file_id, _)| (*file_id, "python"))
            .collect();
        let cache = ParseCache::new();
        is_noise_pattern(&self.members, &sources, &languages, &cache)
    }
}
