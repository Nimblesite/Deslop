//! End-to-end coverage for [OUTPUT-HUMAN-HTML]: the standalone HTML
//! report must carry the design system's real CSS inline.
//!
//! The renderer inlines the site stylesheet into a `<style>` block. A
//! `file://` report has no sibling stylesheets, so any `@import url(...)`
//! left in that inline block resolves to nothing and the whole design
//! system (every `--space-*`, `--font-head`, surface token) silently
//! collapses to the browser's serif default. These tests prove the real
//! design-system CSS is inlined and that no unresolved `@import url(`
//! leaks in its place.

use crate::support::*;

// A renamed (Type-2) C# clone pair guarantees the report renders at
// least one cluster card, so the `<style>` block is exercised on a real
// report body rather than the empty-corpus path.
const CSHARP_A: &str = "namespace Alpha;\n\
                        public sealed class Processor\n\
                        {\n\
                        public int Compute(int input)\n\
                        {\n\
                        int total = 0;\n\
                        for (int i = 0; i < input; i = i + 1) { total = total + i; }\n\
                        return total;\n\
                        }\n\
                        }\n";
const CSHARP_B: &str = "namespace Beta;\n\
                        public sealed class Summer\n\
                        {\n\
                        public int Run(int limit)\n\
                        {\n\
                        int acc = 0;\n\
                        for (int j = 0; j < limit; j = j + 1) { acc = acc + j; }\n\
                        return acc;\n\
                        }\n\
                        }\n";

/// A `:root` token defined only in `site/src/assets/css/base.css`. Its
/// presence inline proves the real design-system stylesheet was bundled
/// rather than the four-line `@import` aggregator. It is the literal
/// declaration (token name + hex), not a `var(...)` reference, so the
/// additive `REPORT_CSS` layer cannot accidentally satisfy it.
const BASE_CSS_MARKER: &str = "--surface-container-low: #1c1b1b;";

/// Runs `deslop` over a freshly-seeded C# clone pair and returns the
/// rendered HTML report.
fn render_html(tmp: &Path) -> Result<String> {
    let scan_root = tmp.join("src");
    fs::create_dir_all(&scan_root)?;
    fs::write(scan_root.join("Alpha.cs"), CSHARP_A)?;
    fs::write(scan_root.join("Beta.cs"), CSHARP_B)?;
    let out = outputs_under(tmp);
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("4")
        .arg("--output")
        .arg(tmp.join("report"))
        .assert()
        .success();
    Ok(fs::read_to_string(&out.html)?)
}

// Implements [OUTPUT-HUMAN-HTML]: the design system's real CSS is
// inlined into the report's `<style>` block, and no unresolved
// `@import url(` aggregator statement leaks in its place.
#[test]
fn html_report_inlines_real_design_system_css() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let html = render_html(tmp.path())?;
    assert!(
        html.contains("<article class=\"cluster-card"),
        "the corpus must produce a rendered cluster card"
    );
    assert!(
        html.contains(BASE_CSS_MARKER),
        "base.css design tokens must be inlined verbatim; \
         expected `{BASE_CSS_MARKER}` in the report's <style> block"
    );
    assert!(
        !html.contains("@import url("),
        "no unresolved @import url( may leak into the inline <style>; \
         a file:// report cannot resolve relative stylesheet imports"
    );
    Ok(())
}
