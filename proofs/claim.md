# Proof Claim Matrix

This document is derived from `proofs/manifest.txt` and states the current proof boundary.

## Verified Boundary

- kernel+builtins

## Refined Claims

- `builtin.tictactoe`: TicTacToe now has an executable model/refinement surface tying runtime init, step, replay, and liveness scaffolding to the proof framework. (proof ids: `ttt_model_init_refines_runtime`, `ttt_model_step_refines_runtime`, `ttt_model_replay_refines_runtime`, `ranked_progress_holds_for_opening_move`, `probabilistic_support_is_finite_and_nonempty`, `session_refinement`, `liveness_model`)

## Implementation-Checked Claims

- `engine.buffer`: Fixed-capacity vectors preserve prefix order and bit-word toggling remains sound. (proof ids: `fixed_vec_push_preserves_prefix_order`, `bit_words_round_trip`)
- `engine.compact`: Compact reward round-trips and schema/bit-width enforcement hold for the implementation helpers. (proof ids: `compact_reward_round_trip`, `compact_observation_words_match_schema`, `compact_reward_bit_width_is_enforced`)
- `engine.rng`: Reference RNG constructor and replay properties hold for the Rust implementation on the verified cases. (proof ids: `rng_state_sanitization_is_total`, `seeded_stream_constructor_handles_reference_cases`, `next_u64_is_repeatable_for_reference_states`)
- `engine.session`: Bounded rewind restoration and replay storage helpers hold for the Rust implementation. (proof ids: `replay_trace_records_steps`, `rewind_restores_prior_state`)
- `engine.env`: The compact environment rejects invalid observation/reward encodings instead of silently accepting them. (proof ids: `env_rejects_invalid_observation_words`, `env_rejects_reward_encoding_that_exceeds_bit_width`)
- `builtin.blackjack`: Blackjack maintains the existing bounded seeded safety/protocol proof surface. (proof ids: `concrete_seed_shuffle_is_a_full_permutation`, `player_observation_hides_opponent_hand_before_terminal`, `initial_observation_contracts_hold_for_concrete_seed`, `stand_action_replays_deterministically_for_seed_17`, `hand_evaluation_matches_busted_flag`)
- `builtin.platformer`: Platformer maintains the existing bounded default-config physics and safety proof surface. (proof ids: `wall_clamps_hold_for_all_edge_positions`, `jump_reward_is_bounded`, `initial_observation_and_world_contracts_hold`, `berry_mask_tracks_trigger_activation`, `clamping_keeps_body_in_bounds`, `oracle_view_matches_world_storage`)

## Model-Only Claims

- `engine.replay-laws`: Replay and canonical observation schema laws are proved at the Verus model level. (proof ids: `session_refinement`)
- `engine.liveness-laws`: Ranking-based termination and finite-support stochastic scaffolding are specified at the Verus model level. (proof ids: `liveness_model`)

## Runtime-Tested Claims

- `render.runtime`: Render/runtime behavior remains tested and benchmarked rather than formally proved.

## Out Of Scope

- `gpu.os`: GPU, OS windowing, and host graphics stacks remain outside the formal proof boundary.

## Assumptions

- `builtin.blackjack`: Current bounded blackjack proofs are tied to concrete seeds and representative hands; they are not universal over all shuffled decks.
- `builtin.platformer`: Current bounded platformer proofs cover the default-config safety surface; full refinement proofs for parameterized physics games remain future work.
- `builtin.tictactoe`: The new liveness claims are about ranking/probabilistic scaffolding on representative traces, not an end-to-end universal fairness proof.
