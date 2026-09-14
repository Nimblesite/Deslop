//! Production embedding-provider registry ([FUSED-EMBED-PROVIDER]).
//!
//! Maps `provider_id` strings to a small factory function that builds
//! a concrete [`EmbeddingProvider`] from a `(model_id, endpoint?)`
//! tuple. Production registers a single entry — `ollama` — so the LSP,
//! MCP, CLI, and VSIX surfaces share one source of truth for which
//! provider ids are selectable. Future bundled providers slot in via
//! [`ProviderRegistry::register`] without touching every transport.
//!
//! The registry is *not* the place to put deterministic test providers.
//! Test-only providers live behind the `test-support` feature in
//! [`crate::embedding::test_support`] and are imported directly by the
//! test that needs them.

use std::{collections::BTreeMap, sync::Arc};

use crate::embedding::{
    ollama::{OllamaProvider, DEFAULT_OLLAMA_ENDPOINT, PROVIDER_ID as OLLAMA_PROVIDER_ID},
    provider::{EmbeddingProvider, ProviderError},
};

/// Factory signature: build a provider from `(model_id, endpoint?)`.
type ProviderFactory = fn(&str, Option<&str>) -> Result<Arc<dyn EmbeddingProvider>, ProviderError>;

/// Registry mapping `provider_id` → factory. Production callers obtain
/// the singleton via [`ProviderRegistry::production`].
#[derive(Debug, Clone)]
pub struct ProviderRegistry {
    /// Ordered `provider_id` → factory map. `BTreeMap` so listing is
    /// stable across runs.
    entries: BTreeMap<String, ProviderFactory>,
}

impl ProviderRegistry {
    /// Returns an empty registry. Call [`Self::register`] to populate.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Returns the production registry. Today this means `ollama`
    /// only — when a second native embedding provider lands it is
    /// added here and every consumer picks it up automatically.
    #[must_use]
    pub fn production() -> Self {
        let mut registry = Self::empty();
        registry.register(OLLAMA_PROVIDER_ID, ollama_factory);
        registry
    }

    /// Registers `factory` under `provider_id`. Re-registering the
    /// same id replaces the previous factory.
    pub fn register(&mut self, provider_id: &str, factory: ProviderFactory) {
        tracing::debug!(provider_id, "embedding_provider_registered");
        let _previous = self.entries.insert(provider_id.to_owned(), factory);
    }

    /// Returns the registered ids in registry order.
    #[must_use]
    pub fn registered_ids(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    /// Returns `true` when `provider_id` is registered.
    #[must_use]
    pub fn contains(&self, provider_id: &str) -> bool {
        self.entries.contains_key(provider_id)
    }

    /// Builds a provider for `provider_id`.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::Unsupported`] when no factory matches
    /// `provider_id` and lifts factory errors into
    /// [`RegistryError::Provider`].
    pub fn build(
        &self,
        provider_id: &str,
        model_id: &str,
        endpoint: Option<&str>,
    ) -> Result<Arc<dyn EmbeddingProvider>, RegistryError> {
        let factory = self.entries.get(provider_id).ok_or_else(|| {
            tracing::warn!(
                requested = provider_id,
                registered = ?self.registered_ids(),
                "embedding_provider_unsupported",
            );
            RegistryError::Unsupported {
                requested: provider_id.to_owned(),
                registered: self.registered_ids(),
            }
        })?;
        tracing::info!(
            provider_id,
            model_id,
            endpoint = endpoint.unwrap_or(DEFAULT_OLLAMA_ENDPOINT),
            "embedding_provider_building",
        );
        factory(model_id, endpoint).map_err(RegistryError::Provider)
    }
}

/// Error variants returned by [`ProviderRegistry::build`].
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    /// `provider_id` is not registered.
    #[error("provider {requested} is not supported (registered: {registered:?})")]
    Unsupported {
        /// Provider id the caller asked for.
        requested: String,
        /// Provider ids the registry knows about.
        registered: Vec<String>,
    },
    /// The factory ran but the provider itself reported an error.
    #[error(transparent)]
    Provider(#[from] ProviderError),
}

/// Production Ollama factory. Connects to `endpoint` (or the default
/// loopback) and probes `model_id`.
fn ollama_factory(
    model_id: &str,
    endpoint: Option<&str>,
) -> Result<Arc<dyn EmbeddingProvider>, ProviderError> {
    let endpoint = endpoint.unwrap_or(DEFAULT_OLLAMA_ENDPOINT);
    let provider = OllamaProvider::connect(endpoint, model_id)?;
    Ok(Arc::new(provider))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_registry_only_contains_ollama() {
        let registry = ProviderRegistry::production();
        let ids = registry.registered_ids();
        assert_eq!(
            ids,
            vec!["ollama".to_owned()],
            "production registry must expose only the Ollama provider",
        );
        assert!(
            registry.contains("ollama"),
            "production registry must contain Ollama provider id",
        );
        assert!(
            !registry.contains("stub"),
            "production registry must not contain stub provider id",
        );
    }

    #[test]
    fn build_unknown_provider_returns_unsupported() {
        let registry = ProviderRegistry::production();
        let outcome = registry.build("stub", "blake3-stub", None);
        assert!(
            matches!(outcome, Err(RegistryError::Unsupported { .. })),
            "stub provider must be Unsupported under the production registry",
        );
        if let Err(RegistryError::Unsupported {
            requested,
            registered,
        }) = outcome
        {
            assert_eq!(requested, "stub");
            assert_eq!(registered, vec!["ollama".to_owned()]);
        }
    }
}
