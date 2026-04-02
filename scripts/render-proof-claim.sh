#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MANIFEST_FILE="${ROOT_DIR}/proofs/manifest.txt"
OUTPUT_FILE="${1:-${ROOT_DIR}/proofs/claim.md}"

heading_for_status() {
  case "$1" in
    refined) echo "Refined Claims" ;;
    checked) echo "Implementation-Checked Claims" ;;
    model) echo "Model-Only Claims" ;;
    runtime) echo "Runtime-Tested Claims" ;;
    out_of_scope) echo "Out Of Scope" ;;
    *) return 1 ;;
  esac
}

{
  echo "# Proof Claim Matrix"
  echo
  echo "This document is derived from \`proofs/manifest.txt\` and states the current proof boundary."
  echo
  echo "## Verified Boundary"
  echo
  awk -F'|' '$1 == "boundary" { printf("- %s\n", $2) }' "$MANIFEST_FILE"

  for status in refined checked model runtime out_of_scope; do
    section="$(heading_for_status "$status")"
    entries="$(
      awk -F'|' -v status="$status" '
        $1 == "claim" && $2 == status {
          printf("- `%s`: %s", $3, $4)
          if (NF >= 5 && length($5) > 0) {
            printf(" (proof ids: ")
            n = split($5, links, ",")
            for (i = 1; i <= n; i++) {
              gsub(/^ +| +$/, "", links[i])
              if (i > 1) {
                printf(", ")
              }
              printf("`%s`", links[i])
            }
            printf(")")
          }
          printf("\n")
        }
      ' "$MANIFEST_FILE"
    )"
    if [[ -n "$entries" ]]; then
      echo
      echo "## ${section}"
      echo
      printf '%s\n' "$entries"
    fi
  done

  assumptions="$(awk -F'|' '$1 == "assumption" { printf("- `%s`: %s\n", $2, $3) }' "$MANIFEST_FILE")"
  if [[ -n "$assumptions" ]]; then
    echo
    echo "## Assumptions"
    echo
    printf '%s\n' "$assumptions"
  fi
} > "$OUTPUT_FILE"
