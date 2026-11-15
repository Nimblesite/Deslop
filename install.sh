#!/usr/bin/env bash
# Installs deslop, deslop-lsp and deslop-mcp from a GitHub release.
# [DEPLOY-DOCS-INSTALLER-FAILCLOSED] Nothing is extracted or installed unless
# the download and its SHA-256 check both succeed.
#
#   curl -fsSL https://raw.githubusercontent.com/Nimblesite/Deslop/main/install.sh | bash
#
# DESLOP_TAG=vX.Y.Z           install that release instead of the latest
# DESLOP_INSTALL_DIR=<dir>    install there without sudo (default: sudo into /usr/local/bin)
# DESLOP_RELEASE_BASE=<url>   release mirror (default: the GitHub releases page)
set -euo pipefail

main() {
  local base tag version platform archive
  base="${DESLOP_RELEASE_BASE:-https://github.com/Nimblesite/Deslop/releases}"
  tag="${DESLOP_TAG:-$(curl -fsSLI -o /dev/null -w '%{url_effective}' "${base}/latest")}"
  tag="${tag##*/}"
  version="${tag#v}"
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64)  platform=linux-x64 ;;
    Linux-aarch64) platform=linux-arm64 ;;
    Darwin-arm64)  platform=macos-arm64 ;;
    Darwin-x86_64) platform=macos-x64 ;;
    *) echo "unsupported platform: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
  esac
  archive="deslop-${version}-${platform}.tar.gz"
  workdir="$(mktemp -d)"
  trap 'rm -rf "$workdir"' EXIT
  cd "$workdir"
  curl -fsSLO "${base}/download/${tag}/${archive}"
  curl -fsSLO "${base}/download/${tag}/${archive}.sha256"
  if command -v sha256sum >/dev/null; then sha256sum -c "${archive}.sha256"; else shasum -a 256 -c "${archive}.sha256"; fi
  tar -xzf "$archive"
  if [ -n "${DESLOP_INSTALL_DIR:-}" ]; then
    mkdir -p "$DESLOP_INSTALL_DIR"
    install -m 755 "deslop-${version}-${platform}"/deslop{,-lsp,-mcp} "$DESLOP_INSTALL_DIR/"
  else
    sudo install -m 755 "deslop-${version}-${platform}"/deslop{,-lsp,-mcp} /usr/local/bin/
  fi
  deslop --version
}

main "$@"
