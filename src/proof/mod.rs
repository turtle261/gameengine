//! Proof-facing assertions and proof-claim surface exported by the crate.

use crate::buffer::Buffer;
use crate::game::Game;
use crate::types::Seed;

/// Human-readable statement of the current proof obligations.
pub const PROOF_CLAIM: &str = include_str!("../../proofs/README.md");

pub use crate::verification::{
    assert_compact_roundtrip, assert_observation_contracts, assert_transition_contracts,
};

/// Runs the canonical generated-game proof surface checks.
pub fn assert_generated_game_surface<G: Game>(
    game: &G,
    state: &G::State,
    actions: &G::JointActionBuf,
    seed: Seed,
) {
    assert_transition_contracts(game, state, actions, seed);
    assert_observation_contracts(game, state);
    if let Some(first) = actions.as_slice().first() {
        assert_compact_roundtrip(game, &first.action);
    }
}
