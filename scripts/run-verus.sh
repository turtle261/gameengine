#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

REQUIRE_VERUS="${REQUIRE_VERUS:-0}"
AUTO_FETCH_VERUS="${AUTO_FETCH_VERUS:-0}"
VERUS_RELEASE_URL="${VERUS_RELEASE_URL:-https://github.com/verus-lang/verus/releases/download/release%2F0.2026.03.28.3390e9a/verus-0.2026.03.28.3390e9a-x86-linux.zip}"

bootstrap_verus_binary() {
  local archive_path
  archive_path="$(mktemp /tmp/verus-release.XXXXXX.zip)"
  local extract_dir
  extract_dir="$(mktemp -d /tmp/verus-release.XXXXXX)"

  echo "[verus] downloading pinned release archive"
  curl -fsSL "$VERUS_RELEASE_URL" -o "$archive_path"
  unzip -q "$archive_path" -d "$extract_dir"

  local extracted
  extracted="$(find "$extract_dir" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
  if [[ -z "$extracted" || ! -x "$extracted/verus" ]]; then
    echo "[verus] archive did not contain an executable verus directory" >&2
    return 1
  fi

  rm -rf ./verus_binary
  mv "$extracted" ./verus_binary
  chmod +x ./verus_binary/verus
  echo "[verus] installed pinned release into ./verus_binary"
}

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
  if [[ "$AUTO_FETCH_VERUS" == "1" || "$REQUIRE_VERUS" == "1" ]]; then
    bootstrap_verus_binary
    VERUS_BIN_PATH="$(resolve_verus_bin)"
  fi
fi

if [[ -z "${VERUS_BIN_PATH:-}" ]]; then
  if [[ "$REQUIRE_VERUS" == "1" ]]; then
    echo "[verus] required but no Verus binary was found (checked VERUS_BIN, ./verus_binary/verus, ./verus_binary, PATH, optional bootstrap)" >&2
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
