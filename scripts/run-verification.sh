#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

export TMPDIR="${TMPDIR:-/var/tmp}"
MODE="${VERIFICATION_MODE:-full}"

COMMON_HARNESSES=(
  bit_words_round_trip
  fixed_vec_push_preserves_prefix_order
  compact_reward_round_trip
  step_outcome_reward_lookup_defaults_to_zero
  replay_trace_records_steps
  rng_state_sanitization_is_total
  seeded_stream_constructor_handles_reference_cases
  next_u64_is_repeatable_for_reference_states
  rewind_restores_prior_state
)

BUILTIN_GAME_HARNESSES=(
  concrete_seed_shuffle_is_a_full_permutation
  player_observation_hides_opponent_hand_before_terminal
  initial_observation_contracts_hold_for_concrete_seed
  stand_action_replays_deterministically_for_seed_17
  hand_evaluation_matches_busted_flag
  legal_actions_are_exactly_empty_cells
  invalid_move_never_mutates_board
)

PHYSICS_HARNESSES=(
  clamping_keeps_body_in_bounds
  oracle_view_matches_world_storage
  wall_clamps_hold_for_all_edge_positions
  jump_reward_is_bounded
  initial_observation_and_world_contracts_hold
  berry_mask_tracks_trigger_activation
)

run_kani_harnesses() {
  local label="$1"
  shift
  local -a extra_args=("$@")

  for harness in "${COMMON_HARNESSES[@]}"; do
    echo "[kani] Running ${label} harness: ${harness}"
    cargo kani --lib "${extra_args[@]}" --harness "${harness}"
  done
}

run_builtin_kani_harnesses() {
  local label="$1"
  shift
  local -a extra_args=("$@")

  for harness in "${BUILTIN_GAME_HARNESSES[@]}"; do
    echo "[kani] Running ${label} harness: ${harness}"
    cargo kani --lib "${extra_args[@]}" --harness "${harness}"
  done
}

run_kani_matrix() {
  if ! command -v cargo-kani >/dev/null 2>&1; then
    echo "[kani] cargo-kani not found; skipping Kani matrix"
    return 0
  fi

  echo "[kani] default headless kernel"
  run_kani_harnesses "default"

  echo "[kani] builtin reference games"
  run_builtin_kani_harnesses "builtin" --features builtin

  echo "[kani] builtin + physics games"
  for harness in "${PHYSICS_HARNESSES[@]}"; do
    echo "[kani] Running builtin+physics harness: ${harness}"
    cargo kani --lib --features "builtin physics" --harness "${harness}"
  done
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
