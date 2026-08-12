//! Ollama embedding provider.
//!
//! Implements [`crate::embedding::EmbeddingProvider`] against the local
//! Ollama HTTP API (`POST /api/embed`, `GET /api/tags`). No TLS
//! — Ollama is a loopback-only service by default, and leaving TLS off
//! keeps the dependency footprint minimal (see `Cargo.toml`).

use std::{thread, time::Duration};

use serde::{Deserialize, Serialize};

use crate::embedding::provider::{
    EmbeddingProvider, EmbeddingSpec, ProviderError, DEFAULT_MAX_INPUT_CHARS,
};

// `OllamaModelInfo` is generated from `docs/models/live-ipc.td` by
// `scripts/typediagram-gen.mjs`. Per CLAUDE.md the IPC model code is
// not stored in git; the binding lives in `crate::wire_generated`.
pub use crate::wire_generated::OllamaModelInfo;

/// Default Ollama endpoint for local runs.
pub const DEFAULT_OLLAMA_ENDPOINT: &str = "http://127.0.0.1:11434";
/// Default embedding model. Chosen because it is a small (137 M
/// parameter, 768-dim) code-friendly embedder that satisfies the
/// ensemble-LLM 2025 finding that smaller embedding models beat
/// larger ones for clone detection, and it is Apache-2.0 licensed.
/// Override with `--embedding-model` when a `nomic-embed-code` or
/// other code-tuned model is pulled locally.
pub const DEFAULT_OLLAMA_MODEL: &str = "nomic-embed-text";
/// Provider registry key.
pub const PROVIDER_ID: &str = "ollama";
/// Expected dimensionality for the default model. Probed at
/// construction time with a short fixed prompt so the spec reflects
/// reality even if the user points at a different model.
const DIMENSION_PROBE_PROMPT: &str = "deslop";
/// Connect and read timeouts for Ollama HTTP calls. Embedding
/// inference is bounded in practice; give enough headroom for cold
/// model loads without blocking the pipeline forever.
const HTTP_TIMEOUT: Duration = Duration::from_secs(60);
/// Hard character cap on any single embed prompt. Ollama returns HTTP
/// 500 ("the input length exceeds the context length") when a prompt
/// overflows the model's context window. `nomic-embed-text` and its
/// peers use a 2048-token window; 6000 chars comfortably undershoots
/// that at ~4 chars/token, and oversized subtrees (generated code,
/// minified files) still contribute a usable prefix instead of
/// aborting the whole pass.
const MAX_EMBED_CHARS: usize = 6000;
/// Number of subtrees sent in one Ollama embedding request. The
/// endpoint accepts array input, but keeping chunks modest avoids
/// oversized JSON bodies and long all-or-nothing retries.
const MAX_BATCH_SIZE: usize = 32;
/// Delay before retrying a failed loopback transport call.
const TRANSPORT_RETRY_DELAY: Duration = Duration::from_millis(25);

/// Retries one transient HTTP transport failure before surfacing it.
fn call_with_transport_retry<T, F>(mut call: F) -> Result<T, ProviderError>
where
    F: FnMut() -> Result<T, ureq::Error>,
{
    match call() {
        Ok(response) => Ok(response),
        Err(first) => {
            tracing::debug!(error = %first, "ollama transport call failed; retrying once");
            thread::sleep(TRANSPORT_RETRY_DELAY);
            call().map_err(|error| provider_unreachable(&error))
        }
    }
}

/// Shared ureq configuration for every Ollama call: one global timeout,
/// and non-success statuses surfaced as responses rather than errors so
/// each call site can attach its own diagnostic.
///
/// Extracted because the identical builder chain sat in `probe`,
/// `post_embeddings`, and `fetch_tags` — Deslop's own `find-similar`
/// flagged it at `fused=1.00` when a fourth copy was about to be
/// written for `/api/show`.
fn ollama_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(HTTP_TIMEOUT))
        .http_status_as_error(false)
        .build()
        .into()
}

/// Maps a transport error into the provider contract.
fn provider_unreachable(error: &ureq::Error) -> ProviderError {
    ProviderError::Unreachable {
        provider_id: PROVIDER_ID.to_owned(),
        message: error.to_string(),
    }
}

/// Ollama provider configured for a specific endpoint + model.
#[derive(Debug, Clone)]
pub struct OllamaProvider {
    /// Endpoint base URL (no trailing slash).
    endpoint: String,
    /// Model identifier as understood by Ollama (e.g.
    /// `"nomic-embed-text"`).
    model: String,
    /// Cached spec built at construction time (identity + dimensions).
    spec: EmbeddingSpec,
    /// Per-input character budget derived from the model's own context
    /// length at construction time.
    max_input_chars: usize,
}

impl OllamaProvider {
    /// Constructs a provider bound to `endpoint` + `model` and probes
    /// the model to discover its dimensionality and version digest.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError::Unreachable`] when Ollama is not
    /// running, [`ProviderError::ProviderFailed`] when the model is
    /// unknown, and [`ProviderError::Malformed`] when the `show` or
    /// `embeddings` endpoints return unexpected shapes.
    pub fn connect(endpoint: &str, model: &str) -> Result<Self, ProviderError> {
        let endpoint = endpoint.trim_end_matches('/').to_owned();
        let model = model.to_owned();
        let version = fetch_model_version(&endpoint, &model)?;
        let dimensions = probe_dimensions(&endpoint, &model)?;
        let spec = EmbeddingSpec {
            provider_id: PROVIDER_ID.to_owned(),
            model_id: model.clone(),
            model_version: version,
            dimensions,
        };
        let max_input_chars = fetch_context_chars(&endpoint, &model);
        Ok(Self {
            endpoint,
            model,
            spec,
            max_input_chars,
        })
    }
}

impl EmbeddingProvider for OllamaProvider {
    fn spec(&self) -> EmbeddingSpec {
        self.spec.clone()
    }

    fn probe(&self) -> Result<(), ProviderError> {
        let url = format!("{}/api/tags", self.endpoint);
        let response = call_with_transport_retry(|| ollama_agent().get(&url).call())?;
        if !response.status().is_success() {
            return Err(ProviderError::ProviderFailed {
                provider_id: PROVIDER_ID.to_owned(),
                message: format!("unexpected status {}", response.status()),
            });
        }
        Ok(())
    }

    fn embed(&self, input: &str) -> Result<Vec<f32>, ProviderError> {
        let embeddings = post_embeddings(&self.endpoint, &self.model, &[input.to_owned()])?;
        let embedding = embeddings
            .into_iter()
            .next()
            .ok_or_else(|| ProviderError::Malformed {
                provider_id: PROVIDER_ID.to_owned(),
                message: "expected one embedding, got none".to_owned(),
            })?;
        ensure_dimensions(&embedding, self.spec.dimensions)?;
        Ok(embedding)
    }

    fn max_batch_size(&self) -> usize {
        MAX_BATCH_SIZE
    }

    fn max_input_chars(&self) -> usize {
        self.max_input_chars
    }

    fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, ProviderError> {
        let embeddings = post_embeddings(&self.endpoint, &self.model, inputs)?;
        if embeddings.len() != inputs.len() {
            return Err(ProviderError::Malformed {
                provider_id: PROVIDER_ID.to_owned(),
                message: format!(
                    "expected {} embeddings, got {}",
                    inputs.len(),
                    embeddings.len()
                ),
            });
        }
        for embedding in &embeddings {
            ensure_dimensions(embedding, self.spec.dimensions)?;
        }
        Ok(embeddings)
    }
}

/// Fetches the model's digest via `GET /api/tags` and matches the
/// requested model by name. `/api/tags` is the only Ollama endpoint
/// that reliably reports a stable content digest across versions;
/// `/api/show` drops the field on some builds.
fn fetch_model_version(endpoint: &str, model: &str) -> Result<String, ProviderError> {
    let parsed = fetch_tags(endpoint)?;
    parsed
        .digest_for(model)
        .as_deref()
        .map(truncate_digest)
        .ok_or_else(|| ProviderError::ProviderFailed {
            provider_id: PROVIDER_ID.to_owned(),
            message: format!("model {model} not installed; run `ollama pull {model}`"),
        })
}

/// Truncates a long digest (64-char SHA-256 hex) to a 12-char prefix
/// so cache directories stay short but remain sensitive to weight
/// changes (12 hex chars = 48 bits, collision-free in practice).
fn truncate_digest(digest: &str) -> String {
    let max = digest.len().min(12);
    digest.chars().take(max).collect()
}

/// Embeds a short fixed prompt to discover the model's output
/// dimensionality. The spec cannot be known ahead of time because
/// users can swap models via `--embedding-model`.
fn probe_dimensions(endpoint: &str, model: &str) -> Result<usize, ProviderError> {
    let embeddings = post_embeddings(endpoint, model, &[DIMENSION_PROBE_PROMPT.to_owned()])?;
    let embedding = embeddings.first().ok_or_else(|| ProviderError::Malformed {
        provider_id: PROVIDER_ID.to_owned(),
        message: "dimension probe returned no embeddings".to_owned(),
    })?;
    Ok(embedding.len())
}

/// Sends one `POST /api/embed` call and returns the parsed
/// embeddings. Centralises truncation, status handling, and error-body
/// capture so `embed`, `embed_batch`, and `probe_dimensions` behave
/// identically.
fn post_embeddings(
    endpoint: &str,
    model: &str,
    inputs: &[String],
) -> Result<Vec<Vec<f32>>, ProviderError> {
    let url = format!("{endpoint}/api/embed");
    let input: Vec<String> = inputs.iter().map(|input| truncate_prompt(input)).collect();
    let body = EmbedRequest {
        model,
        input,
        truncate: true,
    };
    let mut response = call_with_transport_retry(|| ollama_agent().post(&url).send_json(&body))?;
    if !response.status().is_success() {
        let status = response.status();
        let detail = response
            .body_mut()
            .read_to_string()
            .ok()
            .filter(|body| !body.is_empty())
            .map(|body| format!(": {}", body.trim()))
            .unwrap_or_default();
        return Err(ProviderError::ProviderFailed {
            provider_id: PROVIDER_ID.to_owned(),
            message: format!("http status: {status}{detail}"),
        });
    }
    response
        .body_mut()
        .read_json::<EmbedResponse>()
        .map(|parsed| parsed.embeddings)
        .map_err(|err| ProviderError::Malformed {
            provider_id: PROVIDER_ID.to_owned(),
            message: err.to_string(),
        })
}

/// Truncates `input` to at most `MAX_EMBED_CHARS` characters at a UTF-8
/// boundary. Prevents Ollama's "input length exceeds the context
/// length" HTTP 500 for oversized subtrees (generated / minified code).
fn truncate_prompt(input: &str) -> String {
    if input.len() <= MAX_EMBED_CHARS {
        return input.to_owned();
    }
    let end = input
        .char_indices()
        .map(|(idx, _)| idx)
        .take_while(|idx| *idx <= MAX_EMBED_CHARS)
        .last()
        .unwrap_or(0);
    input.get(..end).unwrap_or("").to_owned()
}

/// `POST /api/embed` request body.
#[derive(Debug, Serialize)]
struct EmbedRequest<'a> {
    /// Model name as registered in Ollama.
    model: &'a str,
    /// Texts to embed.
    input: Vec<String>,
    /// Allow Ollama to trim inputs that still exceed the model window.
    truncate: bool,
}

/// `POST /api/embed` response body.
#[derive(Debug, Deserialize)]
struct EmbedResponse {
    /// Dense embedding vectors produced by the model.
    embeddings: Vec<Vec<f32>>,
}

/// Subset of the `GET /api/tags` response we actually use. Each
/// entry carries a `name` (what the user passes to `ollama run X`)
/// and a `digest` (stable content identifier).
#[derive(Debug, Deserialize)]
struct TagsResponse {
    /// Models currently installed on the Ollama server.
    #[serde(default)]
    models: Vec<TagEntry>,
}

/// One installed-model entry.
#[derive(Debug, Deserialize)]
struct TagEntry {
    /// Model name, e.g. `"nomic-embed-text:latest"`.
    #[serde(default)]
    name: String,
    /// Content digest of the packaged weights.
    #[serde(default)]
    digest: String,
    /// Packaged model size in bytes. Surfaced in the VSIX picker so
    /// users can see how much disk each installed model consumes.
    #[serde(default)]
    size: u64,
}

/// Rejects an embedding whose dimensionality does not match `expected`.
fn ensure_dimensions(embedding: &[f32], expected: usize) -> Result<(), ProviderError> {
    if embedding.len() != expected {
        return Err(ProviderError::Malformed {
            provider_id: PROVIDER_ID.to_owned(),
            message: format!("expected {} dims, got {}", expected, embedding.len()),
        });
    }
    Ok(())
}

/// Conservative characters-per-token ratio used to turn a model's
/// token context length into a character budget. Source code tokenises
/// worse than prose, so this stays below the ~4 rule of thumb: a budget
/// that is slightly too small drops a subtree, one that is too large
/// gets silently truncated by Ollama (`truncate: true`) and yields a
/// vector that misrepresents the code ([FUSION-EMBED-PROVIDER]).
const CHARS_PER_TOKEN: usize = 3;

/// Request body for `POST /api/show`.
#[derive(Debug, Serialize)]
struct ShowRequest<'a> {
    /// Model whose metadata is being requested.
    model: &'a str,
}

/// Subset of the `POST /api/show` response we use. `model_info` is a
/// flat map whose context-length key is architecture-prefixed, e.g.
/// `"nomic-bert.context_length"`.
#[derive(Debug, Deserialize)]
struct ShowResponse {
    /// Architecture-keyed model metadata reported by Ollama.
    #[serde(default)]
    model_info: std::collections::BTreeMap<String, serde_json::Value>,
}

/// Best-effort per-input character budget for `model`, derived from the
/// model's own context length. Falls back to [`DEFAULT_MAX_INPUT_CHARS`]
/// whenever `/api/show`, the architecture key, or the context field is
/// missing — the endpoint is incomplete on some Ollama builds, and a
/// conservative budget is always safe.
fn fetch_context_chars(endpoint: &str, model: &str) -> usize {
    context_tokens(endpoint, model)
        .and_then(|tokens| tokens.checked_mul(CHARS_PER_TOKEN))
        .unwrap_or(DEFAULT_MAX_INPUT_CHARS)
}

/// Reads `model_info["<architecture>.context_length"]` from `/api/show`.
fn context_tokens(endpoint: &str, model: &str) -> Option<usize> {
    let show = fetch_show(endpoint, model).ok()?;
    let architecture = show.model_info.get("general.architecture")?.as_str()?;
    let key = format!("{architecture}.context_length");
    usize::try_from(show.model_info.get(&key)?.as_u64()?).ok()
}

/// Fetches the `POST /api/show` payload for `model`.
fn fetch_show(endpoint: &str, model: &str) -> Result<ShowResponse, ProviderError> {
    let url = format!("{endpoint}/api/show");
    let body = ShowRequest { model };
    let mut response = call_with_transport_retry(|| ollama_agent().post(&url).send_json(&body))?;
    if !response.status().is_success() {
        return Err(ProviderError::ProviderFailed {
            provider_id: PROVIDER_ID.to_owned(),
            message: format!("show endpoint failed (status {})", response.status()),
        });
    }
    response
        .body_mut()
        .read_json()
        .map_err(|err| ProviderError::Malformed {
            provider_id: PROVIDER_ID.to_owned(),
            message: err.to_string(),
        })
}

/// Fetches the `GET /api/tags` payload.
fn fetch_tags(endpoint: &str) -> Result<TagsResponse, ProviderError> {
    let url = format!("{endpoint}/api/tags");
    let mut response = call_with_transport_retry(|| ollama_agent().get(&url).call())?;
    if !response.status().is_success() {
        return Err(ProviderError::ProviderFailed {
            provider_id: PROVIDER_ID.to_owned(),
            message: format!("tags endpoint failed (status {})", response.status()),
        });
    }
    response
        .body_mut()
        .read_json()
        .map_err(|err| ProviderError::Malformed {
            provider_id: PROVIDER_ID.to_owned(),
            message: err.to_string(),
        })
}

impl TagsResponse {
    /// Returns the digest for `model`, matching either the full name
    /// (`nomic-embed-text:latest`) or the bare model id
    /// (`nomic-embed-text`). Ollama accepts both forms in `embed`
    /// requests so we must too.
    fn digest_for(&self, model: &str) -> Option<String> {
        self.models
            .iter()
            .find(|entry| entry_matches(entry, model))
            .map(|entry| entry.digest.clone())
    }
}

/// Checks whether `entry.name` matches the user-supplied `model`.
/// Matches by full name first, then by the bare name (tag-stripped)
/// so `foo` matches `foo:latest`.
fn entry_matches(entry: &TagEntry, model: &str) -> bool {
    if entry.name == model {
        return true;
    }
    match entry.name.split_once(':') {
        Some((bare, _tag)) => bare == model,
        None => false,
    }
}

/// Enumerates models currently installed on the Ollama host at
/// `endpoint`. For each model, runs one short embedding probe to
/// classify it as an embedding model — non-embedding models are
/// returned with `is_embedding_model: false` rather than omitted so
/// the VSIX picker can show the full list with an inline badge.
///
/// # Errors
///
/// Returns [`ProviderError::Unreachable`] when `/api/tags` cannot be
/// reached, [`ProviderError::ProviderFailed`] when it responds with
/// a non-2xx status, and [`ProviderError::Malformed`] when the
/// response cannot be parsed.
pub fn list_models(endpoint: &str) -> Result<Vec<OllamaModelInfo>, ProviderError> {
    let endpoint = endpoint.trim_end_matches('/');
    let tags = fetch_tags(endpoint)?;
    let mut out: Vec<OllamaModelInfo> = Vec::with_capacity(tags.models.len());
    for entry in tags.models {
        let bare_id = match entry.name.split_once(':') {
            Some((bare, _tag)) => bare.to_owned(),
            None => entry.name.clone(),
        };
        let digest = truncate_digest(&entry.digest);
        let is_embedding_model = probe_dimensions(endpoint, &bare_id).is_ok_and(|dims| dims > 0);
        out.push(OllamaModelInfo {
            name: entry.name,
            bare_id,
            digest,
            size_bytes: entry.size,
            is_embedding_model,
        });
    }
    Ok(out)
}
