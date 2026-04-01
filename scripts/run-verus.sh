#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

REQUIRE_VERUS="${REQUIRE_VERUS:-0}"

resolve_verus_bin() {
  local requested="${VERUS_BIN:-}"
  local -a candidates=()

  if [[ -n "$requested" ]]; then
    candidates+=("$requested")
  else
    candidates+=("./verus_binary/verus" "./verus_binary" "verus")
  fi

  local candidate
  for candidate in "${candidates[@]}"; do
    if [[ -d "$candidate" && -x "$candidate/verus" ]]; then
      echo "$candidate/verus"
      return 0
    fi
    if [[ -x "$candidate" ]]; then
      echo "$candidate"
      return 0
    fi
    if command -v "$candidate" >/dev/null 2>&1; then
      command -v "$candidate"
      return 0
    fi
  done

  return 1
}

if ! VERUS_BIN_PATH="$(resolve_verus_bin)"; then
  if [[ "$REQUIRE_VERUS" == "1" ]]; then
    echo "[verus] required but no Verus binary was found (checked VERUS_BIN, ./verus_binary/verus, ./verus_binary, PATH)" >&2
    exit 1
  fi
  echo "[verus] no Verus binary found; skipping Verus model checks"
  exit 0
fi

mapfile -t verus_models < <(find proofs/verus -type f -name '*.rs' | sort)

if [[ ${#verus_models[@]} -eq 0 ]]; then
  echo "[verus] no Verus model files found under proofs/verus"
  exit 0
fi

echo "[verus] Using Verus binary: $VERUS_BIN_PATH"
for model in "${verus_models[@]}"; do
  echo "[verus] Checking $model"
  "$VERUS_BIN_PATH" "$model" --crate-type=lib
done

echo "[verus] Completed successfully"
