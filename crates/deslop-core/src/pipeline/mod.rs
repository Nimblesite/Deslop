//! Top-level pipeline orchestration.
//!
//! Glues [PIPELINE-DISCOVER-FILES], [PIPELINE-NORMALIZE-AST],
//! [PIPELINE-FINGERPRINT-MERKLE], [PIPELINE-CLUSTER-EXACT], and
//! [PIPELINE-RANK-WORST-FIRST] together behind a single batch entry
//! point ([`run`]). Exclusion policy ([EXCLUSION-CONFIG]) is applied
//! here: `exclude` filters discovery, `report_hide` flows into the
//! renderer so hidden-only clusters are omitted. Embedding policy
//! ([FUSION-EMBED-PROVIDER]) is applied here too: `EmbeddingSettings`
//! picks a provider/mode and the pass folds `embedding_cos` into
//! candidate pairs before fusion.

mod config;
mod corpus;
mod embedding_batch;
mod embedding_observability;
mod embedding_pass;
mod run;
mod session;
mod signatures;

pub use config::{EmbeddingSettings, PipelineConfig};
pub use corpus::language_ids;
pub use run::{debug_ast_dump, run};
pub use session::PipelineSession;
