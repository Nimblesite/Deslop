//! Fixture for GH #154 — three functions sharing a `(ctx: &mut Ctx)`
//! signature shape collapse to identical structural fingerprints after
//! normalisation. The bodies are wildly different so token_jaccard is
//! zero. The cluster must not surface as duplication.

use std::collections::BTreeMap;

pub struct ProtoCheckCtx<'a> {
    pub messages: &'a mut Vec<String>,
    pub seen_flags: BTreeMap<String, bool>,
    pub counts: (usize, usize),
}

pub fn check_protocol_varargs_kwargs(ctx: &mut ProtoCheckCtx<'_>) -> bool {
    let varargs = ctx
        .seen_flags
        .get("varargs")
        .copied()
        .unwrap_or_default();
    let kwargs = ctx
        .seen_flags
        .get("kwargs")
        .copied()
        .unwrap_or_default();
    if varargs && kwargs {
        ctx.messages
            .push("protocol declares both *args and **kwargs".to_owned());
        return false;
    }
    if varargs {
        ctx.messages
            .push("protocol declares *args without **kwargs".to_owned());
    }
    if kwargs {
        ctx.messages
            .push("protocol declares **kwargs without *args".to_owned());
    }
    true
}

pub fn check_protocol_param_counts(ctx: &mut ProtoCheckCtx<'_>) -> bool {
    let (param_count, max_param_count) = ctx.counts;
    let diff = max_param_count.saturating_sub(param_count);
    if diff > 3 {
        ctx.messages.push(format!(
            "parameter count diverges by {diff} between protocol members",
        ));
        return false;
    }
    if param_count == 0 {
        ctx.messages
            .push("protocol member declares no parameters".to_owned());
    }
    true
}

pub fn check_positional_defaults(ctx: &mut ProtoCheckCtx<'_>) {
    let positional_defaults = ctx
        .seen_flags
        .get("positional_defaults")
        .copied()
        .unwrap_or_default();
    if positional_defaults {
        let summary = format!(
            "{} positional parameters carry implicit defaults",
            ctx.counts.0,
        );
        ctx.messages.push(summary);
        let _ = ctx.seen_flags.insert("positional_defaults".to_owned(), false);
    }
    let _ = ctx.messages.len();
}
