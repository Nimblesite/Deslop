//! Rendering of [`crate::Report`] into derived formats.
//!
//! JSON is canonical — these modules take an already-materialised
//! [`crate::Report`] and produce human- or agent-friendly views over it
//! ([OUTPUT-SCHEMA-JSON]). `--from-report` re-uses these renderers
//! directly so formatting a cached report never re-parses a codebase.

pub mod ast;
pub mod highlight;
pub mod html;
mod html_buckets;
mod html_css;
mod html_escape;
mod html_footer;
pub mod markdown;
pub mod signals;
pub mod text;

pub use ast::render_ast_dump;
pub use html::render_html;
pub use markdown::render_cluster_markdown;
pub use text::render_text;
