# Kani Verification Surface

This crate treats Kani as part of the engine, not an afterthought.

## Pinned Version

- `cargo-kani` / `kani`: `0.67.0`

## Local Commands

```bash
bash scripts/run-verification.sh
```

Run Verus model checks directly:

```bash
bash scripts/run-verus.sh
```

The unified script runs tests, checks, clippy, bench compilation, Kani harnesses, and Verus model checks across three verified layers:

- the default headless kernel,
- the `builtin` reference environments,
- the `builtin + physics` platformer/physics surface.

This keeps failures isolated and avoids monolithic proof runs that are harder to diagnose.

Use concrete seeds for proofs that traverse shuffle-heavy or rejection-sampled RNG paths.
Proofs should target deterministic game properties directly rather than symbolically
exploring an unbounded rejection loop.

## What Is Verified

See [`proofs/claim.md`](claim.md) for a precise verified vs tested vs out-of-scope matrix.

- Fixed-capacity buffer behavior in [`src/buffer.rs`](../src/buffer.rs)
- Reward and replay encoding primitives in [`src/types.rs`](../src/types.rs)
- Compact reward codec soundness in [`src/compact.rs`](../src/compact.rs)
- PRNG replay/fork determinism in [`src/rng.rs`](../src/rng.rs)
- Rollback and replay restoration in [`src/session.rs`](../src/session.rs)
- Game-specific properties in the builtin game modules when `builtin` is enabled
- Physics invariants for the engine-owned 2D world and the platformer environment when
  `builtin` and `physics` are enabled
- Render-input safety claims now include observation decoding and scene-order normalization checks;
  final GPU backend execution remains outside full formal proof scope

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
4. Call the shared helpers in [`src/verification.rs`](../src/verification.rs) for transition and observation contracts.
5. If the game exposes a compact codec, prove action round-trips and reward range correctness.
6. If the game uses the `physics` feature, prove the world invariant before and after every step.
7. If the game is a first-party reference environment, gate it behind `builtin` and add its
  harnesses to [`scripts/run-verification.sh`](../scripts/run-verification.sh).

## Acceptance Rule

A new first-party game is only "verified" when:

- the runtime test suite passes,
- the Kani harnesses pass in the default feature set,
- the Kani harnesses pass in `--features builtin` if it is a builtin reference game,
- the Kani harnesses pass in `--features "builtin physics"` if the game uses the physics subsystem,
- rollback/fork determinism is covered,
- compact encoding is covered when applicable.
