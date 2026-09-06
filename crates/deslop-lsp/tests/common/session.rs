//! One fixture-backed `deslop-lsp` session — the copied workspace, the
//! reaped child, its stdio, and the `initialize` response — so an
//! editor-surface scenario opens in a single call instead of repeating
//! the spawn / handshake / code-action prelude.

use std::{
    io::BufReader,
    process::{ChildStdin, ChildStdout},
};

use anyhow::Result;
use serde_json::Value;
use tower_lsp::lsp_types::Url;

use super::{
    call, call_capturing, code_action_params, handshake, rewrite_offer,
    spawn_lsp_on_fixture_guarded, wait_for_actions, workspace_file_uri, LspGuard,
};

/// Title of the lazily resolved merge offer ([AUTOFIX-MERGE-CODE-ACTION]).
pub const MERGE_OFFER_TITLE: &str = "Merge duplicates into one parameterised helper";

/// Title of the lazily resolved consolidation offer
/// ([AUTOFIX-CONSOLIDATE-CODE-ACTION]).
pub const CONSOLIDATE_OFFER_TITLE: &str =
    "Consolidate identical duplicates into one canonical definition";

/// A live LSP on a copied fixture, past the `initialize` handshake.
///
/// Field order is drop order: the child's stdin closes first so it sees
/// EOF, the guard then reaps it, and only then does the workspace go.
pub struct FixtureSession {
    /// The child's stdin.
    pub stdin: ChildStdin,
    /// The child's buffered stdout.
    pub stdout: BufReader<ChildStdout>,
    /// Reaps the child on drop.
    _guard: LspGuard,
    /// The copied workspace; deleted when the session drops.
    pub workspace: tempfile::TempDir,
    /// The server's `initialize` response.
    pub init: Value,
}

impl FixtureSession {
    /// Copies `fixture`, spawns the LSP on it, and completes the handshake.
    pub fn open(fixture: &str) -> Result<Self> {
        let (workspace, guard, mut stdin, mut stdout) = spawn_lsp_on_fixture_guarded(fixture)?;
        let init = handshake(&mut stdin, &mut stdout)?;
        Ok(Self {
            stdin,
            stdout,
            _guard: guard,
            workspace,
            init,
        })
    }

    /// Sends `method` and returns the paired response.
    pub fn call(&mut self, method: &str, params: &Value) -> Result<Value> {
        call(&mut self.stdin, &mut self.stdout, method, params)
    }

    /// Sends `method` and returns the paired response together with every
    /// server-initiated frame emitted before it.
    pub fn call_capturing(&mut self, method: &str, params: &Value) -> Result<(Value, Vec<Value>)> {
        call_capturing(&mut self.stdin, &mut self.stdout, method, params)
    }

    /// The LSP `Url` of `file_name` inside the workspace.
    pub fn file_uri(&self, file_name: &str) -> Result<Url> {
        workspace_file_uri(self.workspace.path(), file_name)
    }

    /// The `textDocument/codeAction` params over `lines` of `file_name`.
    pub fn code_action_params(&self, file_name: &str, lines: (u32, u32)) -> Result<Value> {
        let uri = self.file_uri(file_name)?;
        Ok(code_action_params(uri.as_str(), lines.0, lines.1))
    }

    /// Polls the code actions over `lines` of `file_name` until the first
    /// analysis pass offers one.
    pub fn wait_for_actions(&mut self, file_name: &str, lines: (u32, u32)) -> Result<Vec<Value>> {
        let params = self.code_action_params(file_name, lines)?;
        wait_for_actions(&mut self.stdin, &mut self.stdout, &params)
    }

    /// The lazily resolved `refactor.rewrite` offer titled `title` over
    /// `lines` of `file_name`.
    pub fn rewrite_offer(
        &mut self,
        file_name: &str,
        lines: (u32, u32),
        title: &str,
    ) -> Result<Value> {
        let actions = self.wait_for_actions(file_name, lines)?;
        rewrite_offer(&actions, title).cloned()
    }
}
