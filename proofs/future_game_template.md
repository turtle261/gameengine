# Future Game Verification Template

Use this checklist when adding a new builtin or first-party game.

## Runtime Checklist

- Add a deterministic smoke test from `init(seed)` through a fixed action trace.
- Add a replay equivalence test using `Session::state_at`, `rewind_to`, and `fork_at`.
- Add a no-allocation hot-path test for direct `step_in_place`.
- Add compact codec round-trip tests if the game implements `CompactGame`.

## `Game` Hook Checklist

Implement and document:

- `state_invariant`
- `action_invariant`
- `player_observation_invariant`
- `spectator_observation_invariant`
- `world_view_invariant`
- `transition_postcondition`

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
        let state = game.init(1);
        let mut actions = FixedVec::<PlayerAction<MyAction>, 1>::default();
        actions.push(PlayerAction { player: 0, action: MyAction::Default }).unwrap();
        crate::verification::assert_transition_contracts(&game, &state, &actions, 1);
    }

    #[kani::proof]
    fn observation_contract_holds_for_initial_state() {
        let game = MyGame::default();
        let state = game.init(1);
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
