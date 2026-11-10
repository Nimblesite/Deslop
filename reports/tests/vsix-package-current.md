# VSIX package verification

Command: `make _vsix-stage-and-package`

Exit status: **0**. Reuses current verified Rust and Node builds.

Source: `e3759a5e9b455d6fc9e4ba7aba5b673158cff856`.

Artifact: `clients/vscode/deslop-live-darwin-arm64.vsix`. SHA-256: `e1c80b66bd2fb19d2cab64eb33427ef0e01e4b0aa9794ca8e8570289c869427c`.

```text
==> Staging bundled binaries into clients/vscode/bin/darwin-arm64
cp shipwright.json clients/vscode/shipwright.json
cd clients/vscode && npm run package
npm notice run deslop-live@0.0.0-dev package
npm notice run node ./scripts/package-vsix.mjs
npm notice run deslop-live@0.0.0-dev build
npm notice run node ./scripts/copy-schema-doc.mjs && node esbuild.mjs
npm notice run deslop-live@0.0.0-dev npx
npm notice run 'vsce' package --no-dependencies --target darwin-arm64 -o deslop-live-darwin-arm64.vsix
 INFO  Files included in the VSIX:
deslop-live-darwin-arm64.vsix
├─ [Content_Types].xml 
├─ extension.vsixmanifest 
└─ extension/
   ├─ LICENSE.txt [1.05 KB]
   ├─ package.json [22.44 KB]
   ├─ readme.md [4.78 KB]
   ├─ shipwright.json [3.19 KB]
   ├─ bin/
   │  └─ darwin-arm64/
   │     ├─ deslop [30.59 MB]
   │     ├─ deslop-lsp [33.29 MB]
   │     └─ deslop-mcp [27.25 MB]
   ├─ dist/
   │  ├─ extension.js [602.76 KB]
   │  └─ schema_doc.md [5.18 KB]
   └─ media/
      ├─ activity-bar.svg [27.9 KB]
      ├─ logo.png [77.69 KB]
      ├─ licenses/
      │  ├─ README.md [1.12 KB]
      │  ├─ osprey.txt [1.08 KB]
      │  └─ seti.txt [1.72 KB]
      └─ webview/
         ├─ cluster.js [212.9 KB]
         ├─ duplication.js [59.59 KB]
         └─ report.js [61.15 KB]

 DONE  Packaged: deslop-live-darwin-arm64.vsix (19 files, 15.07 MB)
Verified extension/dist/schema_doc.md in /Users/christianfindlay/Documents/Code/Deslop/clients/vscode/deslop-live-darwin-arm64.vsix
Verified 3 packaged assets carry no stub provider strings
Verified 19 packaged entries against the shipping allow-list, including 2 manifest-declared assets
Verified deployment manifest and 3 darwin-arm64 VSIX binaries

```
