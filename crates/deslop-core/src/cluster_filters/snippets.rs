//! Snippet construction and per-report CST caching for the cluster-noise
//! filters.
//!
//! A [`Snippet`] pairs a cluster member's raw source with its byte range
//! and a shared, lazily-parsed tree-sitter CST. [`ParseCache`] guarantees
//! each source file is parsed at most once per report, so a large
//! generated file clustered hundreds of ways is never re-parsed per
//! cluster ([CLONE-NOISE-REPARSE-CACHE]). The orchestration that walks
//! these snippets lives in the parent [`super`] module.

use std::{cell::RefCell, collections::HashMap, hash::BuildHasher, rc::Rc};

use super::body_shape::OwnedShapeToken;
use super::{calls::CallShape, contract_index::ContractIndex, polymorphic::OwnedSubject};
use crate::{ast::ByteRange, fingerprint::Fingerprint, lang::shared::parse_source, state::FileId};

/// Bounded per-range memo cells for [`ParseCache`]
/// ([PERF-FLUTTER-TODO-MEMORY]).
mod memos;

/// One re-parsed cluster member: language, raw bytes, the byte range
/// inside `source` that the fingerprint covered, and the originating
/// [`FileId`] so cross-file uniqueness checks do not depend on
/// pointer identity.
pub(crate) struct Snippet<'a> {
    /// Language id used to select the tree-sitter grammar.
    pub(crate) language: &'static str,
    /// Full file source bytes for the member.
    pub(crate) source: &'a [u8],
    /// Byte range covered by the member fingerprint.
    pub(crate) range: ByteRange,
    /// Registry id of the source file containing this member.
    pub(crate) file_id: FileId,
    /// CST for `source`, parsed once per file and shared (via `Rc`) across
    /// every member from the same file. A large file (e.g. a 30k-line
    /// generated FFI binding clustered hundreds of ways) is therefore
    /// parsed at most once per cluster instead of once per filter per
    /// member. `None` when the language has no registered grammar here.
    tree: Option<Rc<tree_sitter::Tree>>,
}

/// How many source bytes the cached CST population may cover
/// ([PERF-FLUTTER-TODO-MEMORY]). A tree-sitter tree costs roughly forty
/// times its source in resident memory, so caching every file of a
/// corpus-scale run is multi-GB — on the Flutter corpus the unbounded
/// cache alone held ~3.2 GB. The budget keeps the covered working set
/// (the files the current clusters actually reference, which arrive
/// with strong locality because components form around files) resident
/// while the long tail re-parses on demand.
pub(crate) const PARSE_TREE_SOURCE_BUDGET_BYTES: usize = 16 * 1024 * 1024;

/// Per-report cache of parsed tree-sitter CSTs keyed by file. A file is
/// parsed at most once per *resident window* regardless of how many
/// clusters reference it — the population is bounded by
/// [`PARSE_TREE_SOURCE_BUDGET_BYTES`] with LRU eviction, so a file can
/// be re-parsed after eviction. Without the cache at all, a large
/// generated file — e.g. a 30k-line FFI binding clustered hundreds of
/// ways — would be re-parsed once per cluster and dominate analysis
/// time ([CLONE-NOISE-REPARSE-CACHE], [PERF-FLUTTER-TODO-MEMORY]).
#[derive(Default)]
pub struct ParseCache {
    /// Lazily-populated map from file id to its parsed CST (or `None`
    /// when the language has no grammar / parsing failed). Bounded by
    /// [`PARSE_TREE_SOURCE_BUDGET_BYTES`] with true-LRU eviction.
    trees: RefCell<HashMap<FileId, Option<Rc<tree_sitter::Tree>>>>,
    /// LRU order of the cached trees: `(file, source bytes)`, most
    /// recently used at the back.
    tree_order: RefCell<std::collections::VecDeque<(FileId, usize)>>,
    /// Sum of the ordered entries' source bytes.
    tree_bytes: std::cell::Cell<usize>,
    /// Lazily-built corpus-wide contract index per language
    /// ([CLONE-NOISE-POLYMORPHIC-CONTRACT]). Built only when a cluster
    /// reaches the contract question, so a report with no same-named
    /// cross-file candidate never pays for it.
    contracts: RefCell<HashMap<&'static str, Rc<ContractIndex>>>,
    /// Kind membership per `(file, byte range)`, fused into one walk
    /// ([PERF-FLUTTER-TODO-CORPUS]). A corpus-scale report asks the
    /// same member ranges repeatedly — across clusters and across the
    /// noise, category, and ranking passes — and each ask used to walk
    /// the member subtree once per kind. One memoised walk per distinct
    /// range replaces all of them.
    field_kinds: RefCell<HashMap<(FileId, usize, usize), FieldKinds>>,
    /// Aggregate cluster-noise counters
    /// ([PERF-FLUTTER-TODO-OBSERVABILITY]): calls, members, fires, and
    /// accumulated time per sub-check, so a corpus-scale run's log says
    /// which filter the time went to.
    noise: RefCell<HashMap<&'static str, NoiseCounters>>,
    /// Enclosing-call shape per member range
    /// ([PERF-FLUTTER-TODO-CORPUS]). The literal-variation filter asks
    /// for the same member ranges once per containing cluster; the
    /// answer is a pure function of `(file, range)`, so it is computed
    /// once however many clusters share the member. Bounded: past the
    /// cap the value is recomputed rather than stored.
    call_shapes: RefCell<HashMap<SnippetKey, Option<Rc<CallShape>>>>,
    /// Covered-statement flag plus in-range call sequence per member
    /// range — one cell because the literal-variation sequence rule
    /// always asks for both, and fusing them halves the memo lookups.
    call_sequences: RefCell<HashMap<SnippetKey, Option<Rc<CallSequence>>>>,
    /// Signature-only body stream per member range.
    signature_shapes: RefCell<HashMap<SnippetKey, Option<Rc<Vec<OwnedShapeToken>>>>>,
    /// Polymorphic subject per member range.
    subjects: RefCell<HashMap<SnippetKey, Option<Rc<OwnedSubject>>>>,
    /// Body-shape digest per enclosing **function** range. Cluster
    /// members nest inside one function (a class of many methods, a
    /// file of many fields), and the digest walk is over the whole
    /// enclosing body — keying by the function collapses every member
    /// of one giant generated function to a single walk
    /// ([PERF-FLUTTER-TODO-CORPUS]).
    body_digests: RefCell<HashMap<SnippetKey, [u8; 32]>>,
}

/// Identity of one member across caches: the file, plus the byte range
/// the fingerprint covers. A file has one language, so this triple
/// fully determines every memoised analysis.
type SnippetKey = (FileId, usize, usize);

/// The fused literal-variation sequence cell: whether every complete
/// statement covered by the range contains a call, and the ordered call
/// shapes fully inside it (`None` when the file has no grammar here).
pub(crate) struct CallSequence {
    /// `every_covered_statement_has_call` verdict.
    pub(crate) all_statements_have_call: bool,
    /// Ordered [`CallShape`]s inside the range.
    pub(crate) shapes: Option<Vec<CallShape>>,
}

/// Running totals for one cluster-noise sub-check.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct NoiseCounters {
    /// Clusters the check ran on.
    pub calls: u64,
    /// Members across those clusters.
    pub members: u64,
    /// Clusters the check suppressed.
    pub fired: u64,
    /// Accumulated wall time in microseconds.
    pub micros: u128,
}

/// Which shape-defining kinds a member subtree contains — the fused
/// answer to the four membership questions the Dart field filter used
/// to ask with four separate walks. One bit per question keeps the
/// cell at a byte.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct FieldKinds {
    /// Set bits: [`FieldKinds::BODY`] for any `function_body`,
    /// [`FieldKinds::FUNCTION_EXPRESSION`] for any
    /// `function_expression`, [`FieldKinds::STATIC_FINAL_LIST`] for any
    /// `static_final_declaration_list`, and
    /// [`FieldKinds::INITIALIZED_IDENTIFIER_LIST`] for any
    /// `initialized_identifier_list`.
    bits: u8,
}

impl FieldKinds {
    /// Bit for `function_body` presence.
    const BODY: u8 = 1 << 0;
    /// Bit for `function_expression` presence.
    const FUNCTION_EXPRESSION: u8 = 1 << 1;
    /// Bit for `static_final_declaration_list` presence.
    const STATIC_FINAL_LIST: u8 = 1 << 2;
    /// Bit for `initialized_identifier_list` presence.
    const INITIALIZED_IDENTIFIER_LIST: u8 = 1 << 3;

    /// Records one kind's presence.
    pub(crate) fn mark(&mut self, kind: &str) {
        self.bits |= match kind {
            "function_body" => Self::BODY,
            "function_expression" => Self::FUNCTION_EXPRESSION,
            "static_final_declaration_list" => Self::STATIC_FINAL_LIST,
            "initialized_identifier_list" => Self::INITIALIZED_IDENTIFIER_LIST,
            _ => 0,
        };
    }

    /// Whether any `function_body` was seen.
    pub(crate) fn has_body(self) -> bool {
        self.bits & Self::BODY != 0
    }

    /// Whether any `function_expression` was seen.
    pub(crate) fn has_function_expression(self) -> bool {
        self.bits & Self::FUNCTION_EXPRESSION != 0
    }

    /// Whether any `static_final_declaration_list` was seen.
    pub(crate) fn has_static_final_list(self) -> bool {
        self.bits & Self::STATIC_FINAL_LIST != 0
    }

    /// Whether any `initialized_identifier_list` was seen.
    pub(crate) fn has_initialized_identifier_list(self) -> bool {
        self.bits & Self::INITIALIZED_IDENTIFIER_LIST != 0
    }
}

impl std::fmt::Debug for ParseCache {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The caches are large interior state; identity is all any
        // Debug consumer needs.
        formatter.write_str("ParseCache")
    }
}

impl ParseCache {
    /// Creates an empty cache scoped to one report render.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one cluster-noise sub-check outcome
    /// ([PERF-FLUTTER-TODO-OBSERVABILITY]).
    pub(crate) fn record_noise(
        &self,
        filter: super::NoiseFilter,
        members: usize,
        fired: bool,
        elapsed: std::time::Duration,
    ) {
        let label = filter.label();
        let mut map = self.noise.borrow_mut();
        let entry = map.entry(label).or_default();
        entry.calls = entry.calls.saturating_add(1);
        entry.members = entry.members.saturating_add(members as u64);
        entry.fired = entry.fired.saturating_add(u64::from(fired));
        entry.micros = entry.micros.saturating_add(elapsed.as_micros());
    }

    /// Emits the aggregate cluster-noise counters once per pass. Sorted
    /// by label so the record is deterministic.
    pub(crate) fn log_noise_totals(&self, stage: &'static str) {
        let mut rows: Vec<(&'static str, NoiseCounters)> = self
            .noise
            .borrow()
            .iter()
            .map(|(label, counters)| (*label, *counters))
            .collect();
        rows.sort_unstable_by_key(|(label, _)| *label);
        for (label, counters) in rows {
            tracing::info!(
                stage,
                filter = label,
                calls = counters.calls,
                members = counters.members,
                fired = counters.fired,
                micros = counters.micros,
                "cluster noise filter totals"
            );
        }
    }

    /// Which shape-defining kinds `node`'s subtree contains, memoised by
    /// `(file, range)` — one walk per distinct member range, however
    /// many clusters and passes ask ([PERF-FLUTTER-TODO-CORPUS]).
    pub(crate) fn dart_field_kinds(
        &self,
        file_id: FileId,
        node: tree_sitter::Node<'_>,
    ) -> FieldKinds {
        let key = (file_id, node.start_byte(), node.end_byte());
        if let Some(hit) = self.field_kinds.borrow().get(&key) {
            return *hit;
        }
        let mut kinds = FieldKinds::default();
        collect_field_kinds(node, &mut kinds);
        let _previous = self.field_kinds.borrow_mut().insert(key, kinds);
        kinds
    }

    /// Returns the cached CST for `file_id`, parsing `source` with the
    /// `language` grammar on first request. `None` when the language has
    /// no registered grammar here or parsing fails.
    pub(crate) fn tree_for(
        &self,
        file_id: FileId,
        language: &'static str,
        source: &[u8],
    ) -> Option<Rc<tree_sitter::Tree>> {
        if let Some(cached) = self.trees.borrow().get(&file_id) {
            self.touch_tree(file_id);
            return cached.clone();
        }
        let parsed = grammar_for(language)
            .as_ref()
            .and_then(|grammar| parse_source(language, grammar, source).ok())
            .map(Rc::new);
        self.insert_tree(file_id, source.len(), parsed.clone());
        parsed
    }

    /// Moves `file_id`'s entry to the back of the LRU order.
    fn touch_tree(&self, file_id: FileId) {
        let mut order = self.tree_order.borrow_mut();
        if let Some(position) = order.iter().position(|(id, _)| *id == file_id) {
            if let Some(entry) = order.remove(position) {
                order.push_back(entry);
            }
        }
    }

    /// Records a freshly parsed tree and evicts least-recently-used
    /// entries until the covered source fits the budget.
    fn insert_tree(&self, file_id: FileId, bytes: usize, tree: Option<Rc<tree_sitter::Tree>>) {
        let _previous = self.trees.borrow_mut().insert(file_id, tree);
        let mut order = self.tree_order.borrow_mut();
        order.push_back((file_id, bytes));
        self.tree_bytes
            .set(self.tree_bytes.get().saturating_add(bytes));
        while self.tree_bytes.get() > PARSE_TREE_SOURCE_BUDGET_BYTES {
            let Some((evict_id, evict_bytes)) = order.pop_front() else {
                break;
            };
            if evict_id == file_id {
                // The insert itself overshot the budget: keep the new
                // entry (a single giant file is a legitimate working
                // set) and stop.
                order.push_front((evict_id, evict_bytes));
                break;
            }
            let _evicted = self.trees.borrow_mut().remove(&evict_id);
            self.tree_bytes
                .set(self.tree_bytes.get().saturating_sub(evict_bytes));
        }
    }

    /// Returns the corpus-wide contract index for `language`, building it
    /// on first request from every same-language file in the report and
    /// reusing the per-file trees this cache already holds.
    pub(super) fn contracts<S: BuildHasher>(
        &self,
        sources: &HashMap<FileId, Vec<u8>>,
        file_languages: &HashMap<FileId, &'static str, S>,
        language: &'static str,
    ) -> Rc<ContractIndex> {
        let cached = self.contracts.borrow().get(language).map(Rc::clone);
        if let Some(index) = cached {
            return index;
        }
        let built = Rc::new(ContractIndex::build(
            sources,
            file_languages,
            language,
            self,
        ));
        let _previous = self
            .contracts
            .borrow_mut()
            .insert(language, Rc::clone(&built));
        built
    }
}

/// Returns a single language id when every member shares it.
pub(crate) fn uniform_language<S: BuildHasher>(
    members: &[Fingerprint],
    file_languages: &HashMap<FileId, &'static str, S>,
) -> Option<&'static str> {
    let first = file_languages.get(&members.first()?.file_id)?;
    if members
        .iter()
        .all(|member| file_languages.get(&member.file_id) == Some(first))
    {
        Some(*first)
    } else {
        None
    }
}

/// Collects one [`Snippet`] per member, returning `None` if any member's
/// source bytes are unavailable. Each distinct source file is parsed at
/// most once for the whole report via `cache`, so downstream filters
/// re-walk a cached CST rather than re-parsing per member or per cluster.
pub(crate) fn collect_snippets<'a>(
    members: &[Fingerprint],
    sources: &'a HashMap<FileId, Vec<u8>>,
    language: &'static str,
    cache: &ParseCache,
) -> Option<Vec<Snippet<'a>>> {
    members
        .iter()
        .map(|member| {
            let source = sources.get(&member.file_id)?;
            let tree = cache.tree_for(member.file_id, language, source);
            Some(Snippet {
                language,
                source: source.as_slice(),
                range: member.byte_range,
                file_id: member.file_id,
                tree,
            })
        })
        .collect()
}

/// Returns the snippet's pre-parsed tree-sitter CST so filters can walk a
/// real CST instead of the normalised one. The tree is parsed once per
/// file in [`collect_snippets`]; this is a cheap `Rc` clone. Returns
/// `None` when the language has no registered grammar here.
pub(crate) fn parse_for(snippet: &Snippet<'_>) -> Option<Rc<tree_sitter::Tree>> {
    snippet.tree.clone()
}

/// Maps a language id to its tree-sitter grammar.
fn grammar_for(language: &str) -> Option<tree_sitter::Language> {
    match language {
        "python" => Some(tree_sitter_python::LANGUAGE.into()),
        "csharp" => Some(tree_sitter_c_sharp::LANGUAGE.into()),
        "rust" => Some(tree_sitter_rust::LANGUAGE.into()),
        "dart" => Some(tree_sitter_dart::LANGUAGE.into()),
        "javascript" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "typescript" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        "fsharp" => Some(tree_sitter_fsharp::LANGUAGE_FSHARP.into()),
        "go" => Some(tree_sitter_go::LANGUAGE.into()),
        _ => None,
    }
}

/// Folds the shape-defining kind membership of `node`'s subtree into
/// `kinds` — the single walk that replaces four per-kind walks.
fn collect_field_kinds(node: tree_sitter::Node<'_>, kinds: &mut FieldKinds) {
    kinds.mark(node.kind());
    if kinds.has_body()
        && kinds.has_function_expression()
        && kinds.has_static_final_list()
        && kinds.has_initialized_identifier_list()
    {
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_field_kinds(child, kinds);
    }
}
