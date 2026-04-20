#!/usr/bin/env bash
# Per-crate Rust line-coverage gate. Reads per-crate thresholds from
# `coverage-thresholds.json` and sums LH/LF from `lcov.info` split by
# crate-directory prefix.
#
# Single source of truth: <repo>/coverage-thresholds.json → .rust.{default_threshold,crates.*}.
# 1% rounding slack matches the VSIX gate in clients/vscode/scripts/check-coverage.mjs.
#
# Usage: scripts/coverage-check.sh <lcov_file> <thresholds_json>
set -uo pipefail

LCOV="${1:-lcov.info}"
THRESHOLDS="${2:-coverage-thresholds.json}"

if [[ ! -f "$LCOV" ]]; then
  echo "FAIL: $LCOV not found"
  exit 1
fi
if [[ ! -f "$THRESHOLDS" ]]; then
  echo "FAIL: $THRESHOLDS not found"
  exit 1
fi

DEFAULT=$(jq -r '.rust.default_threshold' "$THRESHOLDS")
if [[ "$DEFAULT" == "null" || -z "$DEFAULT" ]]; then
  echo "FAIL: $THRESHOLDS missing .rust.default_threshold"
  exit 1
fi

CRATES=(deslop-core deslop deslop-lsp deslop-mcp)
FAILED=0

for crate in "${CRATES[@]}"; do
  threshold=$(jq -r ".rust.crates.\"$crate\" // .rust.default_threshold" "$THRESHOLDS")
  if [[ "$threshold" == "null" || -z "$threshold" ]]; then
    echo "FAIL: no threshold for crate $crate in $THRESHOLDS"
    FAILED=1
    continue
  fi

  # Awk sums LH/LF only for lcov blocks whose SF path contains
  # `crates/<crate>/src/`. The trailing `/src/` disambiguates deslop
  # from deslop-core/deslop-lsp/deslop-mcp (substring collisions).
  counts=$(awk -v crate="crates/$crate/src/" '
    /^SF:/ { in_crate = (index($0, crate) > 0) ? 1 : 0; next }
    /^LH:/ { if (in_crate) lh += substr($0, 4) + 0 }
    /^LF:/ { if (in_crate) lf += substr($0, 4) + 0 }
    /^end_of_record/ { in_crate = 0 }
    END { printf "%d %d", lh, lf }
  ' "$LCOV")
  lh=$(echo "$counts" | awk '{print $1}')
  lf=$(echo "$counts" | awk '{print $2}')

  if [[ "$lf" -eq 0 ]]; then
    echo "FAIL: crate $crate has no covered lines in $LCOV (all files filtered or crate has no tested source)"
    FAILED=1
    continue
  fi

  pct=$(awk -v lh="$lh" -v lf="$lf" 'BEGIN { printf "%.1f", lh / lf * 100 }')
  pass=$(awk -v lh="$lh" -v lf="$lf" -v t="$threshold" 'BEGIN { print (lh / lf * 100 - 1.0 >= t) ? 1 : 0 }')
  if [[ "$pass" -eq 1 ]]; then
    printf "  %-14s %s%% (threshold %s%% + 1%% slack) OK\n" "$crate" "$pct" "$threshold"
  else
    printf "  %-14s %s%% (threshold %s%% + 1%% slack) FAIL\n" "$crate" "$pct" "$threshold"
    FAILED=1
  fi
done

# Workspace-wide roll-up for the top-line summary.
total_counts=$(awk '
  /^LH:/ { lh += substr($0, 4) + 0 }
  /^LF:/ { lf += substr($0, 4) + 0 }
  END { printf "%d %d", lh, lf }
' "$LCOV")
total_lh=$(echo "$total_counts" | awk '{print $1}')
total_lf=$(echo "$total_counts" | awk '{print $2}')
total_pct=$(awk -v lh="$total_lh" -v lf="$total_lf" 'BEGIN { printf "%.1f", lh / lf * 100 }')
echo "Workspace total: $total_pct% ($total_lh/$total_lf lines)"

exit $FAILED
