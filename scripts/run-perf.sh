#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

GAME="${1:-platformer}"
ITERATIONS="${2:-2000000}"
FEATURES="${FEATURES:-builtin physics}"
DATA_FILE="${PERF_DATA_FILE:-/var/tmp/gameengine-perf.data}"

export TMPDIR="${TMPDIR:-/var/tmp}"

if ! command -v perf >/dev/null 2>&1; then
  echo "perf is not installed"
  exit 1
fi

echo "[perf] Building perf probe example"
cargo build --release --example perf_probe --features "$FEATURES"

BIN="target/release/examples/perf_probe"
if [[ ! -x "$BIN" ]]; then
  echo "missing perf probe binary: $BIN"
  exit 1
fi

echo "[perf] perf stat ($GAME, iterations=$ITERATIONS)"
perf stat -e cycles,instructions,branches,branch-misses,cache-references,cache-misses \
  "$BIN" "$GAME" "$ITERATIONS"

echo "[perf] perf record/report ($GAME, iterations=$ITERATIONS)"
perf record -g -o "$DATA_FILE" "$BIN" "$GAME" "$ITERATIONS"
perf report --stdio -i "$DATA_FILE" --sort=dso,symbol | head -n 120
