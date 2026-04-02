# Infotheory Game Engine

`gameengine` is a deterministic, replayable, proof-oriented game/simulation kernel for games treated as mathematical objects.

The kernel is designed around:

`(seed, state, joint_actions) -> (new_state, reward, observations, termination)`

Everything else is layered on top:

- rendering is a derived view,
- human pacing is a presentation concern,
- networking is a transport concern,
- machine control is just another action source,
- replay and rollback are exact because the kernel is deterministic.

## What It Is For

This crate is meant for:

- deterministic game development,
- AIT and AI experiments,
- simulation-heavy search workloads such as MCTS,
- scientific or benchmark environments that need replay fidelity,
- games that benefit from formal reasoning about correctness,
- simulated physical environments.

The target audience is broader than traditional game development: computer scientists, mathematicians, ML/AI researchers, and anyone who needs portable, auditable, replayable environments.

## Design Principles

- Headless by default. The mathematical kernel is the source of truth.
- Deterministic seeded PRNG only. No wall-clock time inside the game core.
- Tick-based simulation. Rendering speed and simulation speed are decoupled.
- Fixed-capacity buffers in the proof-critical path. Hot stepping stays allocation-free.
- Replay, rewind, and fork are first-class.
- Physics is engine-owned, auditable, and deterministic.
- Rendering is additive. A UI cannot change game semantics.
- One canonical observation type per game (`type Obs`), with player/spectator viewpoints encoded from that shared schema.

## Authoring Ergonomics

The core `Game` trait remains available for full control, but single-player environments now have an ergonomic adapter:

- `core::single_player::SinglePlayerGame`

It removes repeated single-player plumbing:

- no manual `player_count = 1` wiring,
- no manual `players_to_act` wiring,
- no manual joint-action extraction boilerplate,
- canonical fixed-capacity reward/joint-action buffer wiring is engine-owned.

This is the intended path for Pong-class ports where the handwritten core should stay close to game math.

Minimal compileable example:

```bash
cargo run --example pong_core
```

## Environment Interface

`core::env::Environment` exposes an infotheory-compatible compact interface:

- `reset(seed)`
- `reset_with_params(seed, params)`
- `step(action_bits) -> EnvStep { observation_bits, reward, terminated, truncated }`

Compact constraints are canonical and centralized in `CompactSpec`:

- observation word count/bit-width validation,
- reward range validation,
- reward bit-width validation.

## Formal Verification Scope

The core engine and builtin reference environments are set up for Kani and Verus checks.

Current proof surface includes:

- fixed-capacity buffers,
- compact codec constraints and roundtrip properties,
- PRNG determinism,
- rollback/replay restoration properties,
- builtin game invariants in the harness matrix,
- engine-owned 2D physics invariants,
- manifest-driven Kani/Verus proof registration,
- executable model/refinement scaffolding for verified games,
- Verus replay/observation/liveness models.

The machine-readable proof boundary lives in [`proofs/manifest.txt`](proofs/manifest.txt).
Claims are intentionally split by status:

- `refined`: backed by Verus model laws and Kani refinement checks,
- `checked`: bounded Kani proofs over the Rust implementation,
- `model`: Verus-only model claims,
- `runtime`: tested/benchmarked behavior,
- `out_of_scope`: explicitly outside the formal boundary.

Games only opt into the stronger surface explicitly:

- implement `proof::ModelGame` and `proof::RefinementWitness`,
- add an explicit `impl proof::VerifiedGame for MyGame {}`,
- register the claim and harness ids in `proofs/manifest.txt`.

Render/runtime behavior is validated by tests and benchmarks; the GPU/driver stack is intentionally outside full formal proof scope.

Run the integrated verification matrix with:

```bash
bash scripts/run-verification.sh
```

Run Verus checks directly:

```bash
bash scripts/run-verus.sh
```

Pin and auto-fetch the CI Verus binary:

```bash
AUTO_FETCH_VERUS=1 REQUIRE_VERUS=1 bash scripts/run-verus.sh
```

Render the human-readable claim matrix from the manifest:

```bash
bash scripts/render-proof-claim.sh
```

## Feature Graph

- `default = []`
  - minimal headless kernel
- `physics`
  - engine-owned deterministic 2D physics
- `builtin`
  - builtin reference environments
- `cli`
  - command-line binary (`gameengine`), depends on `builtin`
- `parallel`
  - batch simulation helpers for independent runs
- `render`
  - additive render/runtime layer

Recommended combinations:

```bash
# headless kernel only
cargo test

# builtin reference environments
cargo test --features builtin

# builtin games plus physics
cargo test --features "builtin physics"

# playable/rendered reference environments
cargo test --features "render builtin physics"
```

## Builtin Reference Games

- `TicTacToe`
- `Blackjack`
- `Platformer`

These are reference environments, not privileged engine special-cases. They demonstrate deterministic game authoring, proof hooks, compact encoding, and render adapters.

## Rendering Model

The render layer is wrapper-first, not kernel-first.

- `--render`: intended player observation/UI path
- `--render-physics`: oracle/developer view of the physics environment

The oracle path can reveal information the player should not see. It exists for debugging, teaching, and diagnostics.

Because the kernel is tick-based, the same game can be:

- trained at compute speed,
- replayed exactly,
- slowed for human-readable pacing,
- or rendered live with AI-driven actions.

## CLI

The CLI is available when `cli` is enabled.

```bash
cargo run --features cli -- list
cargo run --features cli -- play tictactoe --policy human
cargo run --features cli -- replay blackjack --policy script:hit,stand
cargo run --features "cli physics render" -- play platformer --render
cargo run --features "cli physics render" -- play platformer --render-physics --debug-overlay
```

Useful flags:

- `--seed <u64>`
- `--max-steps <usize>`
- `--policy human|random|first|script:...`
- `--render`
- `--render-physics`
- `--ticks-per-second <f64>`
- `--no-vsync`
- `--debug-overlay`

## Rollback And Replay

`SessionKernel`, `DynamicHistory`, and `FixedHistory` support:

- exact trace recording,
- `rewind_to(tick)`,
- `replay_to(tick)`,
- `state_at(tick)`,
- `fork_at(tick)`.

This supports rollback netcode, deterministic multiplayer simulation, offline search, and reproducible experiments.

## WASM

The core library is WASM-compatible. The headless kernel remains portable, and the render stack is structured to compile for WebAssembly.

## Project Direction

The kernel is intentionally shaped to be compatible with Infotheory AIXI interfaces:

- compact `u64` actions/observations,
- `i64` rewards,
- deterministic seeded execution,
- replayable transitions.

## License

This project uses the ISC License (see `LICENSE`).
