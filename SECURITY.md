<!-- agent-pmo:0b21609 -->
# Security Policy

Deslop ships executable developer tooling — the `deslop` / `deslop-lsp` /
`deslop-mcp` binaries (via the VS Code Marketplace / Open VSX VSIX and via
Homebrew/Scoop), and the JetBrains plugin. Because these run inside developers'
editors and CI, we take security reports seriously and respond quickly.

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues,
discussions, or pull requests.**

Report privately through GitHub's **private vulnerability reporting**: go to the
repository's **Security** tab → **Report a vulnerability** (or
<https://github.com/Nimblesite/Deslop/security/advisories/new>). This opens a
private, structured advisory only the maintainers can see.

If you cannot use that channel, email **cftools@nimblesite.co**.

When reporting, please include:

- The type of issue (e.g. injection, path traversal, arbitrary file read/write,
  IPC/transport abuse, secret exposure).
- The affected version(s), file(s), and any relevant configuration.
- Steps to reproduce, ideally a minimal proof of concept.
- The impact: what an attacker can achieve.

## What to Expect

- **Acknowledgement** within **3 business days**.
- An assessment and a remediation plan (or a reasoned decline) within **10 business days**.
- Coordinated disclosure: we will agree a disclosure timeline with you and credit
  you in the advisory unless you prefer to remain anonymous.

## Supported Versions

Deslop is pre-1.0 and ships frequently. Security fixes land on the **latest
released version only** — update your VSIX / CLI to receive them.

| Version | Supported |
| ------- | --------- |
| Latest `0.x` release | ✅ |
| Any older release    | ❌ |

## References

- Add a security policy: <https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/configure-vulnerability-reporting/add-security-policy>
- Configure private vulnerability reporting: <https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/configure-vulnerability-reporting/configure-for-a-repository>
