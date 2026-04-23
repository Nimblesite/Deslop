#!/usr/bin/env bash
# Taxonomy content gate — enforces [CLONE-BUCKETS-DUAL-LABEL] from
# docs/specs/taxonomy.md on product-facing prose (site/src + examples).
#
# Rule: every `Type-N` reference in a product-facing file must have a
# canonical bucket label co-located on the same line or within a
# ±2-line window. Research pages and the generated build output are
# allowlisted — the academic taxonomy is the subject of those docs.
#
# Canonical bucket labels (case-insensitive):
#   - Identical code
#   - Nearly identical code
#   - Loosely similar code
#   - Same behavior, different code
#
# Runs in CI via `make lint`. Exits non-zero with a pointer to every
# offending file:line on failure.

set -euo pipefail
cd "$(dirname "$0")/.."

# Research / academic pages where bare Type-N is the subject matter.
ALLOWLIST=(
  "site/src/docs/research-background.md"
  "site/src/blog/ai-generated-code-duplicate-code.md"
)

# Product-facing file list. `find` keeps this portable across bash
# versions (macOS ships bash 3.2, no globstar or mapfile).
targets=()
while IFS= read -r _file; do
  targets+=("$_file")
done < <(
  find site/src/docs -maxdepth 1 -type f -name '*.md'
  find site/src/blog -maxdepth 1 -type f -name '*.md'
  [[ -f site/src/index.njk ]] && printf '%s\n' site/src/index.njk
  [[ -f examples/README.md ]] && printf '%s\n' examples/README.md
  find examples -type f \( -name '*.cs' -o -name '*.rs' -o -name '*.py' \)
)

# Canonical bucket labels in any surface form (spaced, hyphenated, or
# wrapped across a comment-prefix newline — we flatten whitespace
# before matching so multi-line comments still count).
BUCKETS='[Ii]dentical[- ]code|[Nn]early[- ]identical[- ]code|[Ll]oosely[- ]similar[- ]code|[Ss]ame[- ]behavior,?[- ]different[- ]code|[Ss]ame-behavior'

is_allowlisted() {
  local f=$1
  local a
  for a in "${ALLOWLIST[@]}"; do
    [[ "$f" == "$a" ]] && return 0
  done
  return 1
}

violations=0
for f in "${targets[@]}"; do
  [[ -f "$f" ]] || continue
  is_allowlisted "$f" && continue
  line_nums=$(grep -nE 'Type-[0-9]' -- "$f" 2>/dev/null | cut -d: -f1 || true)
  [[ -z "$line_nums" ]] && continue
  while IFS= read -r lineno; do
    [[ -z "$lineno" ]] && continue
    start=$((lineno - 2))
    (( start < 1 )) && start=1
    end=$((lineno + 2))
    # Flatten the ±2-line window so bucket labels that wrap across a
    # comment-prefix (`// `, `//! `, `///`, `"""`, `#`, `*`) still count.
    ctx=$(sed -n "${start},${end}p" "$f" \
          | tr '\n#*' '   ' \
          | sed -e 's|//!||g' -e 's|///||g' -e 's|//||g' -e 's|"""||g' \
          | tr -s '[:space:]' ' ')
    if ! printf '%s\n' "$ctx" | grep -qE "$BUCKETS"; then
      echo "FAIL: $f:$lineno — bare Type-N without canonical bucket label"
      sed -n "${lineno}p" "$f" | sed 's/^/  > /'
      violations=$((violations + 1))
    fi
  done <<< "$line_nums"
done

# Stale schema guard — the live JSON schema field is `bucket`, not `kind`.
stale_schema=$(grep -rn --include='*.md' --include='*.njk' -E '"kind"[[:space:]]*:[[:space:]]*"Type-' site/src examples 2>/dev/null || true)
if [[ -n "$stale_schema" ]]; then
  echo "FAIL: product-facing file references the stale \"kind\": \"Type-N\" schema (live field is \"bucket\")"
  printf '%s\n' "$stale_schema" | sed 's/^/  > /'
  violations=$((violations + 1))
fi

if (( violations > 0 )); then
  echo
  echo "Taxonomy gate failed: $violations product-facing Type-N reference(s) missing a bucket label."
  echo "See docs/specs/taxonomy.md [CLONE-BUCKETS-DUAL-LABEL]."
  exit 1
fi

echo "Taxonomy gate OK — every product-facing Type-N reference has a bucket label."
