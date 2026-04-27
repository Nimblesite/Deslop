# Release Gates

Deslop uses `deployment-toolkit.json` as the source of truth for release
artifacts and IDE startup checks.

Before publishing, run:

```bash
make deployment-verify
make vsix-package
make jetbrains-package
```

Test entry points remove cargo-installed `deslop`, `deslop-lsp`, and
`deslop-mcp` binaries before running. VSIX tests stage the release binaries
inside `clients/vscode/bin/<platform>/` and clear resolver override
environment variables so activation proves the extension bundle, not PATH.

`make jetbrains-package` currently builds the JetBrains plugin zip without the
product-local archive verifier. Re-enable `scripts/verify-jetbrains-package.mjs`
through GitHub #55 after the local JetBrains Gradle validation path in GitHub
#56 is reliable.

The shared Deployment Toolkit repository is private:
`MelbourneDeveloper/deployment_toolkit`. Agents working from Deployment Toolkit
migration issues must use authenticated `gh` access before relying on its docs
or fixtures:

```bash
gh auth status
gh repo view MelbourneDeveloper/deployment_toolkit --json nameWithOwner,isPrivate,url,defaultBranchRef
```

When Deslop changes its deployment contract, update the private toolkit fixtures
for `fixtures/manifests/deslop.json` and the Rust version-output fixtures in the
same release workflow.
