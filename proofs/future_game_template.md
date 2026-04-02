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

## Proof-Layer Checklist

If the game should participate in the stronger verified surface, also implement:

- `proof::ModelGame`
- `proof::RefinementWitness`
- `proof::VerifiedGame`
- `proof::TerminationWitness` when a ranking argument exists
- `proof::ProbabilisticWitness` when the game has finite-support stochastic choices

Register all Kani and Verus links in `proofs/manifest.txt`.

## Kani Harness Skeleton

```rust
#[cfg(kani)]
mod proofs {
    use super::*;
    use crate::buffer::FixedVec;
    use crate::types::PlayerAction;

    crate::declare_refinement_harnesses!(
        game = MyGame::default(),
        params = MyGame::default().default_params(),
        seed = 1,
        actions = {
            let mut actions = FixedVec::<PlayerAction<MyAction>, 1>::default();
            actions.push(PlayerAction { player: 0, action: MyAction::Default }).unwrap();
            actions
        },
        init = mygame_init_refines_runtime,
        step = mygame_step_refines_runtime,
        replay = mygame_replay_refines_runtime,
    );
}
```

Outside the proof module, add:

```rust
impl crate::proof::VerifiedGame for MyGame {}
```

If your game uses shuffle-heavy setup or rejection-sampled RNG, keep Kani harness seeds concrete unless you have a separately bounded proof wrapper for that RNG path.

## Extra Checks For Physics Games

- Prove the physics world invariant on `init`.
- Prove it again after at least one representative step per action class.
- Prove the logical state and the physics/world view remain synchronized.
