#!/usr/bin/env bash
set -euo pipefail

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

run_harnesses() {
  local label="$1"
  shift
  local -a extra_args=("$@")

  for harness in "${COMMON_HARNESSES[@]}"; do
    echo "Running Kani 0.67.0 ${label} harness: ${harness}"
    cargo kani --lib "${extra_args[@]}" --harness "${harness}"
  done
}

run_builtin_harnesses() {
  local label="$1"
  shift
  local -a extra_args=("$@")

  for harness in "${BUILTIN_GAME_HARNESSES[@]}"; do
    echo "Running Kani 0.67.0 ${label} harness: ${harness}"
    cargo kani --lib "${extra_args[@]}" --harness "${harness}"
  done
}

echo "Running Kani 0.67.0 on the default headless kernel"
run_harnesses "default"

echo "Running Kani 0.67.0 on builtin non-physics games"
run_builtin_harnesses "builtin-games" --features builtin-games

echo "Running Kani 0.67.0 on builtin physics games"
for harness in "${PHYSICS_HARNESSES[@]}"; do
  echo "Running Kani 0.67.0 builtin-games+physics harness: ${harness}"
  cargo kani --lib --features "builtin-games physics" --harness "${harness}"
done
