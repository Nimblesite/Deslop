//! Surface punctuation for pair-evidence values quoted by the refactor
//! gate.

/// How one surface punctuates a `label`/`value` pair and joins pairs.
pub(super) struct PairStyle {
    /// Text between the label and the value.
    separator: &'static str,
    /// Text after the value — Markdown closes its code span here.
    terminator: &'static str,
    /// Text between consecutive pairs.
    joiner: &'static str,
}

/// The refactor gate's refusal line renders verbatim — no markup, so a
/// backtick would show up as a literal character.
pub(super) const PLAIN_STYLE: PairStyle = PairStyle {
    separator: " ",
    terminator: "",
    joiner: " · ",
};

/// Renders `pairs` to two decimal places in `style`.
pub(super) fn render_pairs(pairs: &[(&str, f64)], style: &PairStyle) -> String {
    pairs
        .iter()
        .map(|(label, value)| {
            format!(
                "{label}{separator}{value:.2}{terminator}",
                separator = style.separator,
                terminator = style.terminator,
            )
        })
        .collect::<Vec<_>>()
        .join(style.joiner)
}
