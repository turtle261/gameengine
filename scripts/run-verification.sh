#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

export TMPDIR="${TMPDIR:-/tmp}"
MODE="${VERIFICATION_MODE:-full}"
MANIFEST_FILE="${ROOT_DIR}/proofs/manifest.txt"

run_kani_scope() {
  local scope="$1"
  shift
  local -a extra_args=("$@")

  while IFS='|' read -r kind id harness_scope target; do
    [[ -z "${kind:-}" || "${kind:0:1}" == "#" ]] && continue
    if [[ "$kind" == "kani" && "$harness_scope" == "$scope" ]]; then
      echo "[kani] Running ${scope} harness: ${id}"
      cargo kani --lib "${extra_args[@]}" --harness "${target}"
    fi
  done < "$MANIFEST_FILE"
}

run_kani_matrix() {
  if ! command -v cargo-kani >/dev/null 2>&1; then
    echo "[kani] cargo-kani not found; skipping Kani matrix"
    return 0
  fi

  echo "[kani] default headless kernel"
  run_kani_scope "default"

  echo "[kani] builtin reference games"
  run_kani_scope "builtin" --features builtin

  echo "[kani] builtin + physics games"
  run_kani_scope "builtin+physics" --features "builtin physics"
}

if [[ "$MODE" != "kani-only" ]]; then
  echo "[verify] Running test and check matrix"
  cargo test
  cargo test --features builtin
  cargo test --features "builtin physics"
  cargo test --features parallel
  cargo test --features "render builtin physics"
  cargo check --features render
  cargo check --features "render builtin"
  cargo check --bin gameengine --features cli
  cargo check --bin gameengine --features "cli physics render"
  cargo check --target wasm32-unknown-unknown
  cargo check --target wasm32-unknown-unknown --features physics
  cargo check --target wasm32-unknown-unknown --features "render builtin physics"
  cargo clippy --all-targets --all-features -- -D warnings
  cargo bench --no-run --features "builtin physics"
fi

run_kani_matrix

if [[ "${RUN_VERUS:-1}" == "1" ]]; then
  echo "[verus] Running Verus model checks"
  bash scripts/run-verus.sh
fi

if [[ "${RUN_PERF:-0}" == "1" ]]; then
  echo "[perf] Running perf profile script"
  bash scripts/run-perf.sh
fi

echo "[verify] Completed successfully"
