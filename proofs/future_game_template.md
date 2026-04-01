# Future Game Verification Template

Use this checklist when adding a new builtin or first-party game.

If the game is intended to ship as a first-party reference environment, gate it behind the
`builtin` feature. Rendering stays outside direct GPU
proof scope; only the pure game kernel,
world view, compact codec, and physics hooks belong in the verification checklist.

## Runtime Checklist

- Add a deterministic smoke test from `init_with_params(seed, &params)` through a fixed action trace.
- Add a replay equivalence test using `Session::state_at`, `rewind_to`, and `fork_at`.
- Add a no-allocation hot-path test for direct `step_in_place`.
- Add compact codec round-trip tests for the game action/observation codec hooks.

## `Game` Hook Checklist

Implement and document:

- `state_invariant`
- `action_invariant`
- `player_observation_invariant`
- `spectator_observation_invariant`
- `world_view_invariant`
- `transition_postcondition`

For single-player games, prefer implementing `core::single_player::SinglePlayerGame` and let the engine provide the `Game` adapter wiring.

## Kani Harness Skeleton

```rust
#[cfg(kani)]
mod proofs {
    use super::*;
    use crate::buffer::FixedVec;
    use crate::types::PlayerAction;

    #[kani::proof]
    fn transition_contract_holds_for_representative_step() {
        let game = MyGame::default();
        let state = game.init_with_params(1, &game.default_params());
        let mut actions = FixedVec::<PlayerAction<MyAction>, 1>::default();
        actions.push(PlayerAction { player: 0, action: MyAction::Default }).unwrap();
        crate::verification::assert_transition_contracts(&game, &state, &actions, 1);
    }

    #[kani::proof]
    fn observation_contract_holds_for_initial_state() {
        let game = MyGame::default();
        let state = game.init_with_params(1, &game.default_params());
        crate::verification::assert_observation_contracts(&game, &state);
    }

    #[kani::proof]
    fn compact_round_trip_holds() {
        let game = MyGame::default();
        crate::verification::assert_compact_roundtrip(&game, &MyAction::Default);
    }
}
```

If your game uses shuffle-heavy setup or rejection-sampled RNG, keep Kani harness seeds concrete unless you have a separately bounded proof wrapper for that RNG path.

## Extra Checks For Physics Games

- Prove the physics world invariant on `init`.
- Prove it again after at least one representative step per action class.
- Prove the logical state and the physics/world view remain synchronized.
