//! Proof-facing manifests, model/refinement traits, and reusable harness helpers.

#[macro_use]
mod macros;

pub mod liveness;
pub mod manifest;
pub mod model;
pub mod refinement;

use crate::buffer::Buffer;
use crate::game::Game;
use crate::types::{ReplayStep, Seed};

/// Rendered proof claim matrix generated from the current manifest.
pub const PROOF_CLAIM: &str = include_str!("../../proofs/claim.md");
/// Raw proof manifest used to drive Kani, Verus, and claim reporting.
pub const PROOF_MANIFEST_RAW: &str = include_str!("../../proofs/manifest.txt");

pub use crate::verification::{
    assert_compact_roundtrip, assert_observation_contracts, assert_transition_contracts,
};
pub use liveness::{
    FairnessWitness, FiniteSupportOutcome, ProbabilisticWitness, TerminationWitness,
    assert_finite_support_is_valid, assert_ranked_progress,
};
pub use manifest::{
    ManifestAssumption, ManifestClaim, ManifestHarness, ProofStatus, VerificationManifest,
};
pub use model::{ModelGame, RefinementWitness, SafetyWitness, VerifiedGame};
pub use refinement::{
    assert_model_init_refinement, assert_model_observation_refinement,
    assert_model_replay_refinement, assert_model_step_refinement,
};

/// Returns the parsed proof manifest for this crate.
pub fn verification_manifest() -> &'static VerificationManifest {
    VerificationManifest::current()
}

/// Runs the historical generated-game safety surface checks.
pub fn assert_generated_game_surface<G: Game>(
    game: &G,
    state: &G::State,
    actions: &G::JointActionBuf,
    seed: Seed,
) {
    assert_transition_contracts(game, state, actions, seed);
    assert_observation_contracts(game, state);
    if game.compact_spec().action_count > 0
        && let Some(first) = actions.as_slice().first()
    {
        assert_compact_roundtrip(game, &first.action);
    }
}

/// Runs the strengthened safety/init/step proof surface for an explicitly verified game.
pub fn assert_verified_game_safety_surface<G: VerifiedGame>(
    game: &G,
    state: &G::State,
    actions: &G::JointActionBuf,
    params: &G::Params,
    seed: Seed,
) {
    assert_generated_game_surface(game, state, actions, seed);
    assert_model_init_refinement(game, seed, params);
    assert_model_observation_refinement(game, state);
    assert_model_step_refinement(game, state, actions, seed);
}

/// Runs the replay/rewind refinement surface for an explicitly verified game.
pub fn assert_verified_game_replay_surface<G>(
    game: G,
    params: G::Params,
    seed: Seed,
    trace: &[G::JointActionBuf],
) where
    G: VerifiedGame + Clone,
    ReplayStep<G::JointActionBuf, G::RewardBuf>: Default,
{
    assert_model_replay_refinement(game, seed, params, trace);
}

/// Runs the ranking-based liveness surface for a verified game.
pub fn assert_verified_termination_surface<G: VerifiedGame + TerminationWitness>(
    game: &G,
    state: &G::ModelState,
    actions: &G::JointActionBuf,
    seed: Seed,
) {
    assert_ranked_progress(game, state, actions, seed);
}

/// Runs the finite-support stochastic surface for a verified game.
pub fn assert_verified_probabilistic_surface<G: VerifiedGame + ProbabilisticWitness>(
    game: &G,
    state: &G::ModelState,
    actions: &G::JointActionBuf,
) {
    assert_finite_support_is_valid(game, state, actions);
}

/// Backwards-compatible alias for the safety/init/step surface.
pub fn assert_verified_game_surface<G: VerifiedGame>(
    game: &G,
    state: &G::State,
    actions: &G::JointActionBuf,
    params: &G::Params,
    seed: Seed,
) {
    assert_verified_game_safety_surface(game, state, actions, params, seed);
}
