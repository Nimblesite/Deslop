//! Static CSS for the HTML report.
//!
//! [`SITE_CSS`] is lifted verbatim from the website
//! (`site/src/assets/css/styles.css`) so the report shares the
//! "Kinetic Manuscript" identity ([OUTPUT-HUMAN-HTML]). [`REPORT_CSS`]
//! is the small additive layer of report-only classes (`.snippet`,
//! `.ln`, `.tok-*`, `.cluster-card`, `.run-details`) — additive so the
//! design system stays the source of truth for tokens and surfaces.

/// Design-system stylesheet, bundled at build time directly from the
/// website source. The website's `styles.css` is the single source of
/// truth — `include_str!` re-reads it on every compile, so the report
/// can never lag behind the site. Do NOT copy or paraphrase the CSS
/// here; if a token is missing, add it upstream in
/// `site/src/assets/css/styles.css` and rebuild.
pub const SITE_CSS: &str = include_str!("../../../../site/src/assets/css/styles.css");

/// Report-only CSS additions. Tiny on purpose — anything reusable
/// belongs in the website CSS upstream.
pub const REPORT_CSS: &str = "\
.report-shell{max-width:80rem;margin-inline:auto;padding:var(--space-12) var(--space-6);}\
.report-shell h1{font-size:clamp(2rem,4vw,3rem);font-weight:800;margin-bottom:var(--space-3);letter-spacing:-0.04em;}\
.report-shell .lede{color:var(--on-surface-variant);max-width:48rem;margin-bottom:var(--space-4);}\
.metrics-banner{font-family:var(--font-mono);font-size:0.875rem;padding:var(--space-3) var(--space-4);border-radius:var(--radius-sm);margin:0 0 var(--space-10);border-left:4px solid var(--secondary-container);background:var(--surface-container-low);color:var(--on-surface);}\
.metrics-banner--ok{border-left-color:var(--primary-container);color:var(--primary);}\
.metrics-banner--breached{border-left-color:var(--error,#ff6464);color:var(--error,#ff6464);}\
.metrics-banner--neutral{border-left-color:var(--secondary-container);color:var(--on-surface-variant);}\
.report-shell h2{font-size:1.5rem;font-weight:700;margin:var(--space-12) 0 var(--space-6);letter-spacing:-0.02em;}\
.report-shell .empty{color:var(--on-surface-variant);font-style:italic;}\
.cluster-card{background:var(--surface-container-low);padding:var(--space-6);margin-bottom:var(--space-6);border-radius:var(--radius-sm);border-left:4px solid var(--primary-container);}\
.cluster-card.kind-near{border-left-color:var(--tertiary-container);}\
.cluster-card.kind-weak{border-left-color:var(--secondary-container);}\
.cluster-card__head{display:flex;justify-content:space-between;align-items:flex-start;gap:var(--space-4);margin-bottom:var(--space-3);flex-wrap:wrap;}\
.cluster-card__title{font-family:var(--font-head);font-size:1.125rem;font-weight:700;color:var(--on-surface);letter-spacing:-0.01em;margin:0;}\
.cluster-card__cost{font-family:var(--font-mono);font-size:0.75rem;color:var(--secondary-fixed-dim);text-transform:uppercase;letter-spacing:0.08em;white-space:nowrap;}\
.cluster-card__action{color:var(--on-surface-variant);font-size:0.9375rem;margin:0 0 var(--space-4);}\
.cluster-card__example{font-family:var(--font-mono);font-size:0.75rem;color:var(--secondary-fixed-dim);text-transform:uppercase;letter-spacing:0.08em;margin-bottom:var(--space-2);}\
.snippet{background:var(--surface-container-lowest);color:var(--on-surface);padding:var(--space-4);border-radius:var(--radius-sm);font-family:var(--font-mono);font-size:0.8125rem;line-height:1.55;overflow-x:auto;margin:0 0 var(--space-4);}\
.snippet .ln{display:inline-block;width:3em;color:var(--secondary-fixed-dim);user-select:none;text-align:right;padding-right:var(--space-3);}\
.snippet-missing{color:var(--secondary-fixed-dim);font-style:italic;margin:0 0 var(--space-4);}\
.tok-keyword{color:var(--primary);}\
.tok-type{color:var(--tertiary);}\
.tok-string{color:var(--on-surface-variant);}\
.tok-number{color:var(--tertiary);}\
.tok-comment{color:var(--secondary-fixed-dim);font-style:italic;}\
.tok-function{color:var(--primary);}\
.tok-attribute{color:var(--tertiary);}\
.tok-operator{color:var(--on-surface);}\
.tok-punctuation{color:var(--secondary);}\
.tok-identifier{color:var(--on-surface);}\
.also-list{list-style:none;padding:0;margin:0;display:grid;gap:var(--space-1);}\
.also-list li{font-family:var(--font-mono);font-size:0.8125rem;color:var(--on-surface-variant);}\
.also-list li.is-hidden{color:var(--secondary-fixed-dim);}\
.also-list .also-loc{color:var(--secondary-fixed-dim);margin-left:var(--space-2);}\
.also-toggle{margin-top:var(--space-2);}\
.also-toggle>summary{cursor:pointer;font-family:var(--font-mono);font-size:0.75rem;color:var(--primary);text-transform:uppercase;letter-spacing:0.08em;list-style:none;}\
.also-toggle>summary::-webkit-details-marker{display:none;}\
.run-details{margin-top:var(--space-16);background:var(--surface-container-low);padding:var(--space-4) var(--space-6);border-radius:var(--radius-sm);}\
.run-details>summary{cursor:pointer;font-family:var(--font-mono);font-size:0.75rem;color:var(--secondary-fixed-dim);text-transform:uppercase;letter-spacing:0.1em;list-style:none;}\
.run-details>summary::-webkit-details-marker{display:none;}\
.run-details dl{display:grid;grid-template-columns:max-content 1fr;gap:var(--space-2) var(--space-4);margin:var(--space-4) 0 0;font-family:var(--font-mono);font-size:0.8125rem;}\
.run-details dt{color:var(--secondary-fixed-dim);}\
.run-details dd{margin:0;color:var(--on-surface);}\
.run-details pre{margin-top:var(--space-4);}\
";
