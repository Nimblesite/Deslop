//! The per-file report-hide decision for one render
//! ([EXCLUSION-CONFIG] `report_hide`, [EXCLUSION-GENERATED-BANNER]).
//!
//! A file is hidden when a `report_hide` rule matches its path, or when
//! it opens with a generator's banner. The answer is worked out once per
//! file and read by both the occurrence rows and the duplication
//! percentage, so the two cannot disagree about which files are hidden.

use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Arc, Mutex},
};

use crate::{
    cluster_filters::{locked, ParseCache},
    config::{opens_with_generated_banner, ExclusionConfig},
    lang::{shared::parse_source, LanguageParser},
    pipeline::parser_for_language,
    state::FileId,
};

/// One cluster member's file, as the hide decision sees it.
pub(crate) struct MemberFile<'a> {
    /// Registry identity the decision is remembered under.
    pub(crate) id: FileId,
    /// Absolute path, when the registry still knows the file.
    pub(crate) path: Option<&'a Path>,
    /// Language id the engine analysed the file as.
    pub(crate) language: &'static str,
    /// Source bytes, when the render holds them.
    pub(crate) source: Option<&'a [u8]>,
}

/// Decides which files one render hides.
pub(crate) struct ReportHide<'a> {
    /// `report_hide` path policy.
    exclusion: &'a ExclusionConfig,
    /// The session's language plugins: grammar and comment kinds.
    parsers: &'a [Box<dyn LanguageParser>],
    /// Render-stage trees, reused so a banner check rarely parses
    /// ([CLONE-NOISE-REPARSE-CACHE]).
    parse_cache: &'a ParseCache,
    /// The answer per file, worked out on first ask.
    decided: Mutex<HashMap<FileId, bool>>,
}

impl<'a> ReportHide<'a> {
    /// Starts one render's decisions with nothing decided.
    pub(crate) fn new(
        exclusion: &'a ExclusionConfig,
        parsers: &'a [Box<dyn LanguageParser>],
        parse_cache: &'a ParseCache,
    ) -> Self {
        Self {
            exclusion,
            parsers,
            parse_cache,
            decided: Mutex::new(HashMap::new()),
        }
    }

    /// Whether `file`'s occurrences are hidden: a `report_hide` rule
    /// matches its path, or it opens with a generator's banner.
    pub(crate) fn hides(&self, file: &MemberFile<'_>) -> bool {
        if let Some(known) = locked(&self.decided).get(&file.id) {
            return *known;
        }
        let hidden = self.path_is_hidden(file) || self.is_generated_output(file);
        let _previous = locked(&self.decided).insert(file.id, hidden);
        hidden
    }

    /// Every file decided hidden so far. Complete for a render's clusters
    /// once each has been materialised, which is when the metrics read it.
    pub(crate) fn hidden_files(&self) -> HashSet<FileId> {
        locked(&self.decided)
            .iter()
            .filter_map(|(file_id, hidden)| hidden.then_some(*file_id))
            .collect()
    }

    /// Whether a `report_hide` rule matches the file's path.
    fn path_is_hidden(&self, file: &MemberFile<'_>) -> bool {
        file.path
            .is_some_and(|path| self.exclusion.is_report_hidden(path, file.language))
    }

    /// [EXCLUSION-GENERATED-BANNER] Whether the file opens with a
    /// generator's banner. No source, no plugin for the language, or no
    /// tree all mean "not generated": the file stays visible.
    fn is_generated_output(&self, file: &MemberFile<'_>) -> bool {
        let plugin = parser_for_language(self.parsers, file.language);
        file.source.zip(plugin).is_some_and(|(source, parser)| {
            self.tree(file.id, parser, source)
                .is_some_and(|tree| opens_with_generated_banner(tree.root_node(), source, parser))
        })
    }

    /// The file's tree: the render-stage cache's when it parses this
    /// language, otherwise one parse with the plugin's own grammar.
    fn tree(
        &self,
        file_id: FileId,
        parser: &dyn LanguageParser,
        source: &[u8],
    ) -> Option<Arc<tree_sitter::Tree>> {
        self.parse_cache
            .tree_for(file_id, parser.id(), source)
            .or_else(|| {
                parse_source(parser.id(), &parser.grammar(), source)
                    .ok()
                    .map(Arc::new)
            })
    }
}
