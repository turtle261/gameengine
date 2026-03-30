# gameengine

`gameengine` is a deterministic, replayable, proof-oriented and object-oriented game engine core for games treated as
mathematical objects.

The kernel is designed around the idea that a game is just:

`(seed, state, joint_actions) -> (new_state, reward, observations, termination)`

Everything else is layered on top:

- rendering is a derived view,
- human pacing is a presentation concern,
- networking is a transport concern,
- machine control is just another action source,
- replay and rollback are exact because the kernel is deterministic.

Thus, you can implement a game which is mathematically proven to not have logic bugs if you prove the invariants on it  -- deterministically, anywhere, including in a browser.

## What It Is For

This crate is meant for:

- deterministic game development,
- AIT and AI experiments,
- simulation-heavy search workloads such as MCTS,
- scientific or benchmark environments that need replay fidelity,
- games that benefit from formal reasoning about correctness.
- simulated physical environments

The target audience is broader than traditional game development. The engine is intended to be
useful to computer scientists, mathematicians, ML/AI researchers, and anyone who needs portable,
auditable, replayable environments. 

## Design Principles

- Headless by default. The mathematical kernel is the source of truth.
- Deterministic seeded PRNG only. No wall-clock time inside the game core.
- Tick-based simulation. Rendering speed and simulation speed are decoupled.
- Fixed-capacity buffers in the proof-critical path. Hot stepping stays allocation-free.
- Replay, rewind, and fork are first-class. Rollback netcode can be built on exact state recovery.
- Physics is engine-owned, auditable, and provable -- you may define invariants and prove them with Kani, inheriting the proven correctness and determinism of the Engine. 
- Rendering is additive. A UI can never change the game’s mathematical semantics. Rendering is a function performed upon the observations of a state.

## Formal Verification Scope

The core engine and builtin reference environments are set up for Kani-based verification.

The proof surface covers:

- fixed-capacity buffers,
- compact codecs,
- PRNG determinism,
- rollback/replay restoration,
- game-specific invariants for builtin games,
- engine-owned 2D physics invariants,
- platformer/environment synchronization.

The render stack is intentionally **outside** the proof claim. The claim is that the game kernel and
physics kernel are the mathematical source of truth; the GUI is a derived interface that consumes
verified state. I am not sure if that would be possible to prove. 
If anyone would like to suggest a provable rendering method, I would DEFINITELY be open to consideration.

Run the current proof matrix with:

```bash
bash scripts/run-kani.sh
```

## Feature Graph

- `default = []`
  - minimal headless library kernel
- `physics`
  - engine-owned deterministic 2D physics types and proofs
- `builtin-games`
  - reference environments and the CLI binary
- `parallel`
  - batch-simulation helpers for independent runs
- `render`
  - additive `wgpu`-based render/runtime layer

Recommended combinations:

- headless kernel only:

```bash
cargo test
```

- builtin reference environments:

```bash
cargo test --features builtin-games
```

- builtin games plus physics:

```bash
cargo test --features "builtin-games physics"
```

- playable/rendered reference environments:

```bash
cargo test --features "render builtin-games physics"
```

## Builtin Reference Games

- `TicTacToe`
  - observation-complete turn-based game with deterministic seeded opponent behavior
- `Blackjack`
  - hidden-information card game with seeded shuffle/opponent policy
- `Platformer`
  - simple physics-backed 2D environment with rewards, jump risk, and an oracle physics view

These are reference environments, not privileged engine special-cases. They exist both as examples
of how to implement games with the kernel and as useful ready-made environments for experiments.

Use these as references for how to implement formal verification, how to render a Game Object, etc.

## Rendering Model

The render layer is deliberately wrapper-first, not engine-first.

- `--render` means: render the intended observation/UI path.
- `--render-physics` means: render an explicit oracle/developer view of the underlying physics environment.

That oracle view can reveal more than the player should see. It is useful for debugging,
demonstrations, teaching, and understanding the environment, but it should not be confused with the
fair observation channel.

Because the kernel is tick-based, the same game can be:

- trained as fast as it can be computed,
- replayed exactly,
- slowed down to human-readable speed,
- or rendered live while an AI policy controls the actions.

`--render-physics` will work only on games which use the built-in Physics engine, and will only show that physical environment. Obviously not all games will use 2D physics at all.

`--render` must be implemented manually atop of raw Inputs/Observations -- the library provides 2D Game Rendering abstractions for this,

## CLI

The CLI is available when `builtin-games` is enabled.

```bash
cargo run --features builtin-games -- list
cargo run --features builtin-games -- play tictactoe --policy human
cargo run --features builtin-games -- play blackjack --policy script:hit,stand
cargo run --features "builtin-games physics render" -- play platformer --render
cargo run --features "builtin-games physics render" -- play platformer --render-physics --debug-overlay
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

`SessionKernel` and `FixedHistory` support:

- exact trace recording,
- `rewind_to(tick)`,
- `replay_to(tick)`,
- `state_at(tick)`,
- `fork_at(tick)`.

That makes the engine a clean basis for rollback netcode, deterministic multiplayer simulation,
offline search, and reproducible experiments.

## WASM

The core library is written to remain WASM-compatible. The headless kernel and feature graph are
kept portable, and the render stack is structured so it can compile for WebAssembly. If you see demos on https://infotheory.tech  -- then you can be sure it works beyond compiling.

## Project Direction

The kernel is intentionally shaped to be compatible with [Infotheory](https://github.com/turtle261/infotheory)'s AIXI interfaces:

- `u64` compact actions/observations,
- `i64` rewards,
- deterministic seeded execution,
- zero hidden time,
- replayable state transitions.

Though this may very well be useful for other AI/RL usecases for what is now obvious reasons, given you read this far. 

More creatively, this may be useful for Reservoir Computer design.

You may even call this the "Infotheory Game Engine"


3D Physics engine and Rendering is a goal. It's in the works. 

Intended for games of all types, arbitrarily -- whether it be a mere coinflip, card games, board games, a 3D spaceflight simulation, or a massively multiplayer FPS.

## License
- This is free software, given with the ISC License. This applies to the Software and all associated documentation ("this software").
- Contributing to this specific repository means you agree to submit all contributions under the same Licensing arrangement.
- Don't forget to add your Copyright notice to the LICENSE file.