# Language artwork

`src/types/languageArtwork.json` contains the official Osprey extension icon, resized to 96px for crisp 4× display, and glyph outlines and colors from VS Code's bundled Seti file icon theme. All Osprey syntax flavors share the official cyan bird. Other languages use their VS Code language-ID mapping, including `typescriptreact` for TSX. Unknown files use Seti's default file glyph.

Osprey source is `Nimblesite/osprey/src/toolchain/vscode-extension/icon.png`, SHA-256 `94d994718eda3fd5967905ad2863ec7ef6d30caa6c84305a8e6be51ad8de0984`. Its MIT license is in `osprey.txt`.

Seti source is VS Code's `extensions/theme-seti/icons/seti.woff` and `vs-seti-icon-theme.json`, referencing upstream Seti commit `2d6c5e68b4ded73c92dac291845ee44e1182d511`. Its third-party notice and MIT license are in `seti.txt`. The Osprey image is resized with Lanczos resampling while preserving transparency. Outlines are scaled uniformly to the 24px canvas, with their source colors preserved.

Regenerate with `uv run --no-project scripts/import-language-artwork.py /path/to/theme-seti/icons /path/to/osprey/icon.png` from `clients/vscode`.
