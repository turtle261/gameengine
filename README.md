# Infotheory Game Engine

`gameengine` is a deterministic, replayable, proof-oriented environment kernel.

The engine is organized so game authors focus on game mathematics first:

`(seed, state, joint_actions) -> (new_state, reward, canonical observation bits, termination)`

Everything else (session/replay, compact codecs, registry/CLI wiring, proof helpers,
physics/render integration) is engine-owned and reusable.

## Rewrite Architecture

The crate remains a single artifact and is library-first by default.

- `src/lib.rs`
  - canonical public API and feature-gated exports
- `src/core/`
  - core deterministic interfaces and wrappers
  - canonical observation trait (`Observe` + `Observer`)
  - infotheory-ready environment wrapper (`Environment`, `EnvStep`, `BitPacket`)
  - explicit fast vs checked stepper wrappers
- `src/proof/`
  - proof-facing helper surface and claim document wiring
- `src/physics.rs`
  - deterministic physics world + contact generation
  - hybrid broadphase: tiny-world fast path + scalable sweep-and-prune path
- `src/render/`
  - optional retained-mode renderer
  - hot path updated to avoid per-frame cache/scene cloning where possible
- `src/builtin/`
  - builtin implementation namespace
  - concrete game implementations under `src/builtin/tictactoe/`, `src/builtin/blackjack/`, and `src/builtin/platformer/`
- `src/registry/`
  - static game descriptor registry used by the CLI
- `src/cli/`
  - optional registry-backed CLI integration
- `src/bin/gameengine.rs`
  - binary entrypoint (feature-gated)

## Canonical Observation + Env Surface

The rewrite introduces a single canonical observation surface for consumers:

- `core::observe::Observe`
  - one observation schema type per game (`type Obs`)
  - observer-aware extraction (`Observer::Player`, `Observer::Spectator`)
  - canonical compact encoding
- `core::env::Environment`
  - `reset(seed)`
  - `step(action_bits)`
  - returns `EnvStep { observation_bits, reward, terminated, truncated }`

This is designed to map directly to infotheory-style environment loops.

## Feature Graph

- `default = []`
  - minimal headless library
- `proof`
  - proof helper surface exports
- `physics`
  - deterministic 2D physics
- `builtin`
  - builtin reference environments
- `cli`
  - command-line frontend (`gameengine` binary), depends on `builtin`
- `parallel`
  - parallel replay helpers
- `render`
  - optional retained-mode renderer/runtime

## Verification

Run the unified verification workflow:

```bash
bash scripts/run-verification.sh
```

This script runs:

- test/check matrix across core feature combinations,
- clippy (`-D warnings`),
- benchmark compilation,
- Kani harness matrix (when `cargo-kani` is installed),
- Verus model checks (when `verus` is installed).

## Performance Tooling

Benchmarks:

```bash
cargo bench --bench step_throughput --features "builtin physics"
cargo bench --bench kernel_hotpaths --features "builtin physics"
```

Perf profiling (Linux):

```bash
bash scripts/run-perf.sh platformer 3000000
```

The perf probe targets release-mode stepping loops without Criterion analysis overhead,
so hotspot attribution is meaningful.

## CLI

The CLI is registry-backed: game listing and dispatch come from `src/registry/mod.rs`.
Adding a game now requires a descriptor registration rather than editing multiple match sites.

```bash
cargo run --features cli -- list
cargo run --features cli -- play tictactoe --policy human
cargo run --features cli -- replay blackjack --policy script:hit,stand
cargo run --features "cli physics render" -- play platformer --render
cargo run --features "cli physics render" -- play platformer --render-physics --debug-overlay
```

## Proof Claim Scope

Proof claim details live in:

- `proofs/README.md`

Current claim includes deterministic kernel contracts, compact codec properties,
replay/rewind restoration, and physics invariants for supported feature sets.
GPU backend execution remains outside full formal proof scope.

## License

ISC.
