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

Pin and auto-fetch the exact Verus release used by CI:

```bash
AUTO_FETCH_VERUS=1 REQUIRE_VERUS=1 bash scripts/run-verus.sh
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

See [`proofs/manifest.txt`](manifest.txt) for the machine-readable proof boundary and
[`proofs/claim.md`](claim.md) for the rendered human-readable matrix.

- Fixed-capacity buffer behavior in [`src/buffer.rs`](../src/buffer.rs)
- Reward and replay encoding primitives in [`src/types.rs`](../src/types.rs)
- Compact reward codec soundness in [`src/compact.rs`](../src/compact.rs)
- PRNG replay/fork determinism in [`src/rng.rs`](../src/rng.rs)
- Rollback and replay restoration in [`src/session.rs`](../src/session.rs)
- Game-specific properties in the builtin game modules when `builtin` is enabled
- Physics invariants for the engine-owned 2D world and the platformer environment when
  `builtin` and `physics` are enabled
- Verus model lemmas in [`proofs/verus/session_refinement.rs`](verus/session_refinement.rs)
  and [`proofs/verus/liveness_model.rs`](verus/liveness_model.rs)
  for replay fold refinement, canonical observation-schema constraints, and liveness scaffolding
- Render/input/runtime behavior is covered by tests and benchmarks; it is not currently
  claimed as fully formally verified

## Verification Pattern For New Games

1. Implement the `Game` proof hooks:
   - `state_invariant`
   - `action_invariant`
   - `player_observation_invariant`
   - `spectator_observation_invariant`
   - `world_view_invariant`
   - `transition_postcondition`
2. Add runtime tests for determinism, replay, compact codecs, and rollback if the game uses sessions.
3. Implement the proof-layer traits in [`src/proof/model.rs`](../src/proof/model.rs) when the
   game opts into executable model/refinement checks.
   Add an explicit `impl proof::VerifiedGame for MyGame {}` only after the stronger surface is intentional.
4. Add `#[cfg(kani)]` proof harnesses in the game module, preferably through the proof macros.
5. Call the shared helpers in [`src/verification.rs`](../src/verification.rs) for transition and observation contracts.
6. If the game exposes a compact codec, prove action round-trips and reward range correctness.
7. If the game uses the `physics` feature, prove the world invariant before and after every step.
8. If the game is a first-party reference environment, register its claims and harnesses in
   [`proofs/manifest.txt`](manifest.txt) so the verification scripts and claim docs stay aligned.

## Acceptance Rule

A new first-party game is only "verified" when:

- the runtime test suite passes,
- the Kani harnesses pass in the default feature set,
- the Kani harnesses pass in `--features builtin` if it is a builtin reference game,
- the Kani harnesses pass in `--features "builtin physics"` if the game uses the physics subsystem,
- rollback/fork determinism is covered,
- compact encoding is covered when applicable.
