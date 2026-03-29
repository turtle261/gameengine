# Kani Verification Surface

This crate treats Kani as part of the engine, not an afterthought.

## Pinned Version

- `cargo-kani` / `kani`: `0.67.0`

## Local Commands

```bash
bash scripts/run-kani.sh
```

The script runs the proof surface harness-by-harness for the default kernel, the `physics`
feature set, and the physics-only harnesses. This keeps failures isolated and avoids
monolithic proof runs that are harder to diagnose.

Use concrete seeds for proofs that traverse shuffle-heavy or rejection-sampled RNG paths.
Proofs should target deterministic game properties directly rather than symbolically
exploring an unbounded rejection loop.

## What Is Verified

- Fixed-capacity buffer behavior in [`src/buffer.rs`](/home/theo/dev/gameengine/src/buffer.rs)
- Reward and replay encoding primitives in [`src/types.rs`](/home/theo/dev/gameengine/src/types.rs)
- Compact reward codec soundness in [`src/compact.rs`](/home/theo/dev/gameengine/src/compact.rs)
- PRNG replay/fork determinism in [`src/rng.rs`](/home/theo/dev/gameengine/src/rng.rs)
- Rollback and replay restoration in [`src/session.rs`](/home/theo/dev/gameengine/src/session.rs)
- Game-specific properties in the builtin game modules
- Physics invariants for the engine-owned 2D world when `physics` is enabled

## Verification Pattern For New Games

1. Implement the `Game` proof hooks:
   - `state_invariant`
   - `action_invariant`
   - `player_observation_invariant`
   - `spectator_observation_invariant`
   - `world_view_invariant`
   - `transition_postcondition`
2. Add runtime tests for determinism, replay, compact codecs, and rollback if the game uses sessions.
3. Add `#[cfg(kani)]` proof harnesses in the game module.
4. Call the shared helpers in [`src/verification.rs`](/home/theo/dev/gameengine/src/verification.rs) for transition and observation contracts.
5. If the game exposes a compact codec, prove action round-trips and reward range correctness.
6. If the game uses the `physics` feature, prove the world invariant before and after every step.

## Acceptance Rule

A new first-party game is only "verified" when:

- the runtime test suite passes,
- the Kani harnesses pass in the default feature set,
- the Kani harnesses pass in `--features physics` if the game uses the physics subsystem,
- rollback/fork determinism is covered,
- compact encoding is covered when applicable.
