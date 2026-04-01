# Proof Claim Matrix

This document states what `gameengine` currently claims as formally verified, what is tested,
and what is intentionally outside full proof scope.

## Formally Verified (Kani Harness Surface)

- Fixed-capacity containers and bit-word primitives.
- Compact reward codec round-trips and range soundness.
- Deterministic RNG construction and replay properties.
- Replay rewind restoration for bounded history configurations.
- Builtin game invariants included in harness matrix.
- Physics invariants and platformer synchronization harnesses for `builtin + physics`.

## Verified By Runtime Tests + Property Tests

- Seeded determinism and replay equivalence in integration tests.
- Compact traces and stable hashes for golden trajectories.
- Allocation-free stepping on core builtin hot paths.
- Render presenter scene emission and driver progression behavior.

## In Scope But Not Fully Formalized Yet

- Registry-level descriptor integrity and dispatch consistency.
- Higher-level CLI orchestration and policy script UX behavior.
- Richer progress/liveness obligations beyond bounded checks.

## Out of Full Formal Scope

- GPU/driver execution details (`wgpu`, OS windowing, platform graphics stack).
- Host runtime behavior outside deterministic kernel contract.

## Execution Entry Point

Run the consolidated verification surface with:

```bash
bash scripts/run-verification.sh
```
