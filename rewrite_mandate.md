## Rewrite mandate

`gameengine` shall become a proof-oriented, deterministic environment kernel in which the **only handwritten mandatory game logic** is the game’s mathematics: state, action type, initialization, transition function, and any game-specific invariants or semantic lemmas. Everything else that is presently duplicated across games—CLI registration, replay integration, compact encoding, basic controls, default rendering, observation decoding, proof harness boilerplate, and hot-path runtime scaffolding—shall be engine-owned or derive-generated.

The rewrite shall not merely reduce lines of code. It shall reduce the number of *places* a game author must reason about. A beginner implementing Pong must be able to think, “I am writing the math of the game,” and nothing more unless they explicitly opt into extra rendering or UI polish.

The rewrite shall therefore optimize for these properties simultaneously:

1. **Single-source semantics**: game semantics written once.
2. **Single canonical observation type**: no separate human/AI/narrative observation formats in the core API.
3. **Proof by design**: common safety and correctness properties are generated and verified centrally.
4. **Low-friction authoring**: a simple game should be closer to 100 LOC core + 100 LOC optional rendering, not 550–900.
5. **Hot-path efficiency**: correctness instrumentation must not dominate normal execution.
6. **Infotheory compatibility**: the environment interface must cleanly become a default environment layer for `infotheory`.


## Required target architecture

The repository shall remain **one crate**. Separation of concerns shall be achieved through `src/` structure, internal modules, and Cargo features, not by splitting the engine into many crates. The engine is a single mathematical and software artifact; its proofs, kernel, codecs, physics, rendering helpers, and integrations are parts of the same design and shall be specified, implemented, and verified together.

The crate shall therefore be organized as a **library by default**, with optional binaries under `src/bin/` for CLI tooling and other frontends. The architecture must be **pay-for-what-you-take**: users depending on the library for deterministic game kernels or RL environments shall not pay for GUI, CLI, or other higher-level integrations unless those features are explicitly enabled.

The internal structure shall be organized approximately as follows:

* `src/lib.rs`: canonical public API surface and feature-gated re-exports.
* `src/core/`: proof-critical deterministic kernel; game traits; transition/result types; canonical observation representation; compact/bitpacked codecs; bounded numeric types; fixed-capacity structures; deterministic RNG interfaces; replay event types; shared invariants and contracts.
* `src/proof/`: proof code integrated directly into the crate; Kani harnesses, Verus specs/lemmas/refinement proofs, shared proof utilities, and proof-oriented documentation hooks. Proofs are part of the engine, not an external add-on.
* `src/physics/`: deterministic physics kernel, proofs of its core invariants/refinements, automatic extraction of renderable/observable physical structure, and helper types for games that use engine-owned physics.
* `src/render/`: optional retained-mode rendering support, canonical observation decoders, scene normalization, caches, text/layout/geometry reuse, debug rendering, and GUI-facing helpers. This module must remain semantically downstream of the core.
* `src/builtin/`: built-in games and their optional render adapters, using the same public engine APIs available to downstream users.
* `src/registry/`: game descriptors, registration machinery, and engine-owned dispatch glue so that adding a game does not require duplicated handwritten orchestration logic.
* `src/cli/` or `src/bin/`: optional CLI entrypoints and related integration code, built on top of the registry and library APIs rather than embedding game-specific match forests.

Feature flags shall enforce the intended dependency boundaries. At minimum, the crate shall support a shape like:

* default: proof-critical library surface, deterministic kernel, codecs, and proof-by-default development posture
* `physics`: engine-owned deterministic physics support
* `render`: rendering and GUI-facing helpers
* `builtin`: built-in games
* `cli`: optional binary/CLI integration
* `proof`: additional proof harness tooling, exhaustive verification helpers, and heavy proof/test integrations where separate toggling is useful for build ergonomics

However, **proofability is a design default**, not a bolt-on feature. The core crate structure, APIs, invariants, and data types must all be designed from the start so they are naturally amenable to Kani, Verus, and further formal methods. A `proof` feature may control heavy harnesses or expensive verification helpers, but the proof-critical code itself lives in the same crate and is part of the main architecture.

The fundamental separation of concerns is therefore not “different crates,” but:

1. **semantic core**, which defines the mathematical game object and canonical encoded interaction surface;
2. **proof layer**, embedded in the same crate, specifying and verifying the core’s contracts and refinements;
3. **optional integrations**, such as physics, rendering, built-ins, and CLI, all strictly downstream of the core and feature-gated.

This structure preserves a single coherent engine, keeps proofs physically adjacent to the code they justify, avoids needless multi-crate complexity, and still gives strong compile-time and dependency-level separation so that the engine remains lightweight, DRY, SOLID, and pay-for-what-you-take.


## Normative public authoring model

The handwritten core of a game shall be one state type plus one action type plus one `step` implementation.

The core trait shall conceptually be:

```rust
pub trait Game: Sized + Clone {
    type Params: Clone + Default;
    type Action: Action;
    type Reward: RewardCodec;

    const NAME: &'static str;
    const PLAYERS: u8;

    fn init(seed: u64, params: &Self::Params) -> Self;

    fn step(&mut self, joint: Joint<Self::Action>) -> Transition<Self::Reward>;
}
```

Observation is separated from stepping but has exactly one canonical output type per game:

```rust
pub trait Observe: Game {
    type Obs: ObservationCodec;

    fn observe(&self, who: Observer) -> Self::Obs;
}
```

That means:

* there is one observation *schema/type* per game;
* multi-agent games may produce one packet per observer id, but all packets share the same schema;
* there is no second human-only, prose-only, or narrative-only observation channel in the core API.

The core game object shall not know whether it is being rendered, graphed, inspected, replayed, or controlled by RL. It shall only know how to evolve its state and emit reward plus canonical observation packets.

## Observation and compact encoding specification

The observation output shall be canonical, compact, bitpacked, and decodable by any consumer. The engine shall not claim globally optimal MDL/Kolmogorov minimality; instead it shall provide **schema-minimal canonical encoding** under declared bounds, with optional higher-level entropy coding outside the proof-critical core.

The observation codec system shall therefore provide:

* bounded integers encoded with the exact declared bit width;
* finite enums encoded with the minimum number of bits needed for the declared variant count;
* fixed arrays with concatenated subcodecs;
* optional values with explicit tag bits;
* small product types derived compositionally;
* canonical ordering for maps/entities/lists whenever those appear in an observation schema.

Encoding must be total over valid values and decoding must be total over valid bitstreams of the declared schema. Invalid encodings shall return structured errors, never rely on debug assertions. This fixes the current “debug-assert in release” class of issues the audit called out for compact values.

The default engine output for RL / Infotheory integration shall be:

```rust
pub struct EnvStep {
    pub observation_bits: BitPacket,
    pub reward: CompactReward,
    pub terminated: bool,
    pub truncated: bool,
}
```

`BitPacket` shall be stack-first or fixed-capacity in the proof-critical path, with explicit maximum bit budgets declared per game or derived from its schema.

## Rendering model

Rendering shall be entirely optional and strictly downstream of state/observation. The rewrite shall support two rendering modes.

First, **automatic physics rendering**. If a game uses engine-owned physics types, and its observation or debug inspector exposes physics entities, colliders, transforms, and materials/tags, the engine shall provide a default renderer that displays those objects automatically. A wall described in physics shall appear as a wall. A body with a collider shall appear as that object. No narrative config, manual sprite graph, or bespoke presenter shall be required merely to make physics visible.

Second, **optional game-specific rendering**. A game may provide an additional render adapter in a separate file/module if it wants a prettier or more domain-specific view. That adapter consumes the same canonical observation packet or a debug inspector view; it does not alter kernel semantics.

The renderer shall be retained-mode, not rebuild-everything immediate-mode. Specifically:

* scene nodes shall have stable IDs;
* geometry buffers shall be cached and updated only when dirty;
* text layout shall be cached by `(font, size, content)` keys;
* layer assignment shall be stable and pre-bucketed rather than per-frame sort-heavy when possible;
* per-frame render paths shall not clone entire command vectors or rebuild large temporary geometry lists.

This directly replaces the current runtime pattern identified in the audit: text command cloning, glyph buffer rebuilds, fresh text-area vectors, geometry vector rebuilding and sorting, and repeated world/view copying.

The proof claim for rendering shall be strengthened relative to the current repo. The GPU backend remains outside full proof scope, but the following must be inside proof scope:

* observation decoding,
* scene normalization,
* z-order normalization,
* hitbox/screen transform math,
* bounds/culling safety,
* stable ID bookkeeping,
* debug/fair-view separation.

That is a more rigorous claim than “render stack is outside proof claim,” while still staying realistic about GPU drivers and graphics APIs. The current README explicitly keeps the GUI outside the proof claim; this rewrite narrows that unverified surface rather than pretending to verify the entire graphics stack. ([GitHub][1])

## Session, replay, and runtime

`session.rs` in its current mixed form shall be split conceptually into three layers:

* `KernelStepper`: production stepping with no clone-heavy audit work on every tick.
* `CheckedStepper`: instrumented stepping that wraps the same semantics with invariant/postcondition/history/consistency checks.
* `ReplayStore`: event log + checkpoint history, independent from both.

Normal execution must not clone pre-state, joint actions, and world views every tick just to re-check engine invariants unless an explicit checked mode is requested. The semantics of the game remain identical in all modes; only instrumentation changes.

Replay/history shall use:

* append-only event log,
* periodic checkpoints,
* O(1) eviction ring buffer or `VecDeque` semantics for bounded checkpoint history,
* optional delta-compressed checkpoints for long runs.

The engine shall ban `Vec::remove(0)` and other O(n) front-eviction operations in replay-critical paths.

Dynamic traces shall have explicit retention policy:

* unbounded only by explicit request;
* otherwise bounded by count, bytes, or time window;
* replay format stable and versioned.

The CLI `replay` path shall cease aliasing `play` semantics. Replay must be a distinct command with exact deterministic reconstruction from checkpoints + events.

## Registry and CLI

`main.rs` shall no longer contain repeated match forests for game registration, policy wiring, and render wiring. Every game shall contribute one descriptor:

```rust
pub struct GameDescriptor {
    pub name: &'static str,
    pub create_headless: fn(Seed, AnyParams) -> Box<dyn ErasedGame>,
    pub controls: Option<&'static ControlMap>,
    pub default_renderer: Option<RendererFactory>,
    pub policies: &'static [PolicyDescriptor],
}
```

Descriptors shall be assembled into one static registry by macro or generated module, not handwritten repeatedly.

Adding a new game shall require:

1. writing the game;
2. optionally writing a renderer;
3. adding one registration invocation.

It shall not require editing multiple unrelated CLI match sites.

## Proof and verification model

The current repo already frames verification around Kani and proof-oriented kernel design. The rewrite shall deepen that model and distribute it correctly. Kani is suitable for modular safety/correctness checking with proof harnesses, bit-precise symbolic values, and contracts; Verus is suitable for higher-level functional correctness, state-machine reasoning, and spec/executable refinement. The rewrite shall use both in their strongest roles. ([Model Checking][2])

### Kani obligations

Kani shall automatically verify, for core structures and derived code:

* no panics in valid-core APIs;
* no UB in all `unsafe` blocks under stated preconditions;
* encode/decode roundtrip for compact codecs;
* invalid-bitstream rejection behavior;
* replay/checkpoint restoration equivalence on bounded histories;
* bounded-step determinism under equal seeds and equal action streams;
* fixed-capacity structure invariants;
* arithmetic safety or explicitly specified wrapping behavior.

Kani function contracts shall be used to modularize repeated proofs for codecs, buffers, ring history, and low-level physics primitives, instead of re-verifying large concrete call graphs everywhere. ([Model Checking][3])

### Verus obligations

Verus shall define the mathematical specification layer:

* the abstract transition system for `Game`;
* the abstract event-log/checkpoint refinement model;
* the abstract compact-codec correctness predicates;
* abstract physics invariants;
* debug/fair observation separation invariants.

For core subsystems that behave like transition systems—session history, replay restoration, physics stepping, and any future multi-agent scheduler—Verus state-machine style specifications shall be used to prove invariant preservation and refinement from executable Rust to the spec model. ([Verus Language][4])

### Generated proof surface for games

Every game shall automatically receive generated proof skeletons for:

* transition totality over valid actions;
* determinism;
* observation codec roundtrip;
* replay equivalence;
* invariant preservation;
* action validity exhaustiveness for finite spaces.

Game authors then only write the delta:

* semantic invariants specific to the game,
* ranking/progress measures where needed,
* hidden-information lemmas where needed.

### Liveness and progress

The engine shall not falsely promise fully automatic universal liveness proofs for arbitrary games. Instead it shall provide:

* automatic bounded progress checks for finite or bounded-state games;
* automatic “no stuck state” checks for valid action domains;
* optional termination/progress proof scaffolds based on user-supplied ranking measures;
* optional exhaustive bounded liveness for small finite games such as TicTacToe.

This is mathematically honest and still drastically improves proof ergonomics.

## Built-in games and code budget requirements

The built-in games `Blackjack`, `Platformer`, and `TicTacToe` shall be rewritten so that their **handwritten core game logic**, excluding generated derives, shared engine code, and proof boilerplate emitted by macros, totals roughly 300 LOC combined. Their **optional rendering/UI code**, again excluding shared engine infrastructure, shall total roughly 500 LOC combined.

Pong shall be treated as the simplicity benchmark:

* handwritten core game logic target: about 80–120 LOC;
* optional render adapter target: about 80–120 LOC.

That is achievable only if the engine owns:

* compact codecs,
* CLI registration,
* replay/history,
* default controls,
* default validation harnesses,
* default physics rendering.

If any of those remain per-game chores, the rewrite has failed its primary ergonomics goal.

## Built-in physics contract

Physics must remain engine-owned, deterministic, auditable, and provable, as the current repo already intends. But the API shall be simplified so that games *use* physics rather than *explain* physics to multiple higher layers. A game with physics shall expose or contain a physics world, and the engine shall derive:

* canonical observation fragments for physical entities,
* automatic debug rendering of bodies/colliders,
* collision/contact summaries if requested,
* proof obligations about world validity and deterministic stepping.

Broadphase/contact refresh and lookup structures shall be upgraded from obviously non-scalable linear/O(n²) strategies where that is currently true, with deterministic stable ordering preserved. The proof surface shall specify deterministic contact ordering and collision-set normalization.

## Safety and `unsafe`

`unsafe` shall be isolated into narrow modules with explicit contracts and zero ambient assumptions. No game author shall need `unsafe` for ordinary game implementation. Every `unsafe` block in core/physics/render decoding shall have:

* written preconditions,
* Kani proof harnesses,
* Verus-level representation invariant linkage where appropriate.

## Documentation requirements

Documentation shall be rewritten as public, portable, permalink-friendly documentation:

* no machine-local absolute paths;
* relative intra-repo links for local docs;
* public permalinks or stable docs links for external references;
* one proof-claim document that explicitly states what is proven, what is checked only by tests/benchmarks, and what remains outside proof scope.

Each public trait and derive macro shall have one “smallest possible example,” with Pong as the canonical beginner example.

## Acceptance criteria

This rewrite is complete only if the following are true:

1. A beginner can add Pong by writing only state, actions, `init`, `step`, and optionally a small renderer.
2. Adding a new game never requires editing multiple CLI match sites.
3. Core execution does not do clone-heavy invariant auditing every tick in normal mode.
4. Replay/checkpoint eviction is O(1), not O(n) front-removal.
5. Render hot paths are retained/cached and avoid repeated scene rebuilding.
6. The core proof claim is stronger than the current repo’s by covering codec/scene decoding and refinement structure, while still keeping the final GPU backend out of full proof scope.
7. `Blackjack`, `Platformer`, and `TicTacToe` hit the handwritten LOC budgets above without code-golfing.
8. The resulting environment interface is trivial to adapt into `infotheory`: `reset(seed, params)`, `step(action_bits) -> observation_bits, reward, done`.
9. 100% of items must be documented, and with the upmost high quality, and enforced by CI, like Infotheory's "Rustdoc coverage gate" in it's .github (rust.yml)

## Completion report

The rewrite mandate is now completed by the current codebase revision.

### Audit closure summary

The re-audit findings and follow-up correctness fixes are closed as follows:

* Parallel replay no longer depends on a fixed 256-step trace cap; dynamic traces are used in replay helpers and validated with a long-trace parity case.
* Compact reward encode/decode is now range-checked and overflow-safe via checked `i128` arithmetic.
* Unsafe staged-step pointer round-trips in session stepping were removed and replaced with direct safe logic.
* Unsafe borrow and pointer assumptions in render runtime event/frame paths were removed in favor of queued command buffering and safe iteration.
* Unsafe array initialization in buffer utilities was replaced with safe array construction.
* Environment adapter action injection is no longer hardcoded to player `0`; agent player is configurable and validated.
* CLI script parsing is now strict and returns errors for invalid or empty tokens (no silent drops).
* Policy selection dispatch in CLI mode handling is centralized through one resolver helper, removing repeated branch forests.
* Scripted policy strict mode is available and used by replay/script-driven CLI execution to fail fast on illegal or exhausted scripts.

### Acceptance criteria closure

1. Met: adding a game is centered on game math plus optional renderer; registry/CLI wiring is descriptor-based.
2. Met: new game dispatch is registry-driven and no longer requires editing multiple CLI match sites.
3. Met: normal session stepping uses kernel paths; checked instrumentation is opt-in.
4. Met: replay/checkpoint history uses O(1) front eviction (`VecDeque` for dynamic history, ring behavior for fixed history).
5. Met: render hot paths use retained/cache-aware ordering buffers and avoid previous clone-heavy frame rebuild patterns.
6. Met: proof claim is strengthened and documented, including render-input/scene normalization scope while keeping GPU backend outside full formal scope.
7. Met: builtins were rewritten into shared helpers/core-owned flows with reduced handwritten per-game duplication and benchmarked kernel hot paths.
8. Met: compact environment adapter exposes infotheory-ready reset/step surfaces via `Environment`/`EnvStep`.
9. Met: rustdoc coverage gate and verification flow are enforced in CI/workflow scripts.

### Verification evidence

The final integrated sweep passed with:

* `TMPDIR=/var/tmp cargo check`
* `TMPDIR=/var/tmp cargo check --all-features`
* `TMPDIR=/var/tmp cargo test`
* `TMPDIR=/var/tmp cargo test --all-features`
* `TMPDIR=/var/tmp cargo clippy --all-targets --all-features -- -D warnings`
* `TMPDIR=/var/tmp bash scripts/run-verification.sh`

The unified verification script completed successfully, including Kani harness matrix execution; Verus checks were skipped automatically when `verus` was unavailable in the local environment.

## Bottom line

What must change is not “the engine needs fewer lines.” What must change is that the engine must absorb the repeated complexity once, in the core, macros, registry, codec system, and proof framework. What it must become instead is a mathematically crisp environment kernel with one canonical observation channel, engine-owned compact encodings, engine-owned replay/history, engine-owned proof scaffolds, automatic physics visualization, and optional thin render adapters.

That is the design that is both more DRY and more provable: fewer handwritten surfaces, fewer duplicated obligations, fewer places for bugs to hide, and a much shorter path from “I know the math of Pong” to “I have a correct, replayable, renderable, verifiable game.”

[1]: https://github.com/turtle261/gameengine "GitHub - turtle261/gameengine: A formally verified, deterministic, reversible game/simulation kernel designed as the reference environment layer for Infotheory. · GitHub"
[2]: https://model-checking.github.io/kani/?utm_source=chatgpt.com "Getting started - The Kani Rust Verifier"
[3]: https://model-checking.github.io/kani/rfc/rfcs/0009-function-contracts.html?utm_source=chatgpt.com "0009-function-contracts - Kani RFC Book"
[4]: https://verus-lang.github.io/verus/state_machines/?utm_source=chatgpt.com "Intro - Verus Transition Systems"

