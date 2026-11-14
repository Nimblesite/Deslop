//! [LSP-EMBEDDING-CONSENT] The backend's startup provider, in process.
//!
//! The editor normally starts the LSP with embeddings off and selects a
//! model later; a client may instead hand a mode, provider, model and
//! endpoint to `LspBackend::new_with_config` up front. Each choice has one
//! outcome: off and a reachable provider start normally, an unreachable
//! provider degrades to no embeddings rather than failing the editor
//! (whether the user asked for `auto` or `required`), and a provider id
//! the registry does not know is refused by name.

use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use deslop_core::{
    embedding::{EmbeddingMode, DEFAULT_OLLAMA_MODEL, DEFAULT_PROVIDER_ID},
    live::{transport::IpcMode, LiveError},
};
use deslop_lsp::backend::{LspBackend, LspEmbeddingConfig};
use tower_lsp::{Client, LspService};

use crate::{common::copy_fixture, mock_ollama::MockOllama};

const FIXTURE: &str = "csharp-small";
const MIN_NODES: u32 = 30;
/// Loopback port nothing listens on: the connection is refused at once.
const UNREACHABLE_ENDPOINT: &str = "http://127.0.0.1:1";
const UNKNOWN_PROVIDER: &str = "no-such-provider";

/// A live client handle, taken from a throwaway default backend.
fn client(workspace: &std::path::Path) -> Result<Client> {
    let captured: Arc<Mutex<Option<Client>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&captured);
    let root = workspace.to_path_buf();
    let (_service, _socket) = LspService::build(move |client| {
        if let Ok(mut held) = slot.lock() {
            *held = Some(client.clone());
        }
        match LspBackend::new_with_defaults(client, root.clone(), MIN_NODES) {
            Ok(backend) => backend,
            Err(error) => {
                tracing::error!(%error, "startup embedding test backend failed to initialise");
                std::process::exit(1);
            }
        }
    })
    .finish();
    let held = captured
        .lock()
        .map_err(|_| anyhow!("client slot poisoned"))?
        .clone();
    held.ok_or_else(|| anyhow!("the service built no client"))
}

fn config(mode: EmbeddingMode, provider_id: &str, endpoint: &str) -> LspEmbeddingConfig {
    LspEmbeddingConfig {
        mode,
        provider_id: provider_id.to_owned(),
        model_id: DEFAULT_OLLAMA_MODEL.to_owned(),
        endpoint: endpoint.to_owned(),
    }
}

fn start(
    workspace: &std::path::Path,
    embedding: &LspEmbeddingConfig,
) -> Result<Result<LspBackend, LiveError>> {
    Ok(LspBackend::new_with_config(
        client(workspace)?,
        workspace.to_path_buf(),
        MIN_NODES,
        embedding,
        IpcMode::platform_default(),
    ))
}

/// Every startup embedding choice, each with its one outcome.
#[tokio::test(flavor = "current_thread")]
async fn each_startup_embedding_choice_starts_degrades_or_is_refused() -> Result<()> {
    let workspace = copy_fixture(FIXTURE)?;
    let root = workspace.path();
    let ollama = MockOllama::spawn()?;

    for (label, embedding) in [
        (
            "off",
            config(
                EmbeddingMode::Off,
                DEFAULT_PROVIDER_ID,
                UNREACHABLE_ENDPOINT,
            ),
        ),
        (
            "auto, reachable",
            config(EmbeddingMode::Auto, DEFAULT_PROVIDER_ID, ollama.endpoint()),
        ),
        (
            "auto, unreachable",
            config(
                EmbeddingMode::Auto,
                DEFAULT_PROVIDER_ID,
                UNREACHABLE_ENDPOINT,
            ),
        ),
        (
            "required, unreachable",
            config(
                EmbeddingMode::Required,
                DEFAULT_PROVIDER_ID,
                UNREACHABLE_ENDPOINT,
            ),
        ),
    ] {
        assert!(
            start(root, &embedding)?.is_ok(),
            "{label}: the backend starts and the editor keeps working"
        );
    }

    for mode in [EmbeddingMode::Auto, EmbeddingMode::Required] {
        let refused = start(root, &config(mode, UNKNOWN_PROVIDER, ollama.endpoint()))?;
        match refused {
            Err(LiveError::UnsupportedProvider {
                requested,
                registered,
            }) => {
                assert_eq!(requested, UNKNOWN_PROVIDER, "{mode:?}");
                assert!(
                    registered.iter().any(|id| id == DEFAULT_PROVIDER_ID),
                    "{mode:?}: the refusal names the providers that exist: {registered:?}"
                );
            }
            Err(other) => anyhow::bail!("{mode:?}: refused for the wrong reason: {other}"),
            Ok(_) => anyhow::bail!("{mode:?}: an unknown provider id must be refused"),
        }
    }
    Ok(())
}
