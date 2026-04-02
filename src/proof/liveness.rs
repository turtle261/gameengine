//! Liveness-oriented proof scaffolding layered on top of executable model semantics.

use core::fmt::Debug;

use crate::buffer::Buffer;
use crate::proof::model::ModelGame;
use crate::rng::DeterministicRng;
use crate::types::{StepOutcome, Termination};

/// Ranking-function based termination witness over the executable model.
pub trait TerminationWitness: ModelGame {
    /// Returns a natural-number rank that must decrease on non-terminal progress steps.
    fn model_rank(&self, state: &Self::ModelState) -> u64;

    /// Returns whether terminal states are exactly the rank-zero states.
    fn terminal_rank_is_exact(&self, state: &Self::ModelState) -> bool {
        self.model_is_terminal(state) == (self.model_rank(state) == 0)
    }
}

/// Checks the ranking-function progress obligation for one model transition.
pub fn assert_ranked_progress<G: TerminationWitness>(
    game: &G,
    pre: &G::ModelState,
    actions: &G::JointActionBuf,
    seed: u64,
) {
    let mut post = pre.clone();
    let mut rng = DeterministicRng::from_seed_and_stream(seed, 777);
    let mut outcome = StepOutcome::<G::RewardBuf>::default();
    let pre_rank = game.model_rank(pre);
    game.model_step_in_place(&mut post, actions, &mut rng, &mut outcome);

    assert!(game.terminal_rank_is_exact(pre));
    assert!(game.terminal_rank_is_exact(&post));

    if !game.model_is_terminal(pre) {
        assert!(game.model_is_terminal(&post) || game.model_rank(&post) < pre_rank);
    } else {
        assert_eq!(game.model_rank(&post), 0);
        assert!(outcome.termination.is_terminal());
    }
}

/// Declarative fairness witness scaffold for future game-specific obligations.
pub trait FairnessWitness: ModelGame {
    /// Returns the fairness assumptions required by the game's liveness claims.
    fn fairness_assumptions(&self) -> &'static [&'static str] {
        &[]
    }
}

/// One weighted model outcome in a finite-support stochastic step.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct FiniteSupportOutcome<S, R> {
    /// Successor state for this support point.
    pub state: S,
    /// Reward buffer emitted for this support point.
    pub rewards: R,
    /// Termination status emitted for this support point.
    pub termination: Termination,
    /// Relative support weight for this outcome.
    pub weight: u64,
}

/// Finite-support stochastic witness scaffold for probabilistic liveness proofs.
pub trait ProbabilisticWitness: ModelGame {
    /// Buffer type that stores all finite-support outcomes for one model step.
    type SupportBuf: Buffer<Item = FiniteSupportOutcome<Self::ModelState, Self::RewardBuf>>
        + Clone
        + Debug
        + Default
        + Eq
        + PartialEq;

    /// Enumerates the finite support of one model step for the given state and actions.
    fn model_step_support(
        &self,
        state: &Self::ModelState,
        actions: &Self::JointActionBuf,
        out: &mut Self::SupportBuf,
    );
}

/// Checks that a probabilistic witness exposes a non-empty, positive-weight finite support.
pub fn assert_finite_support_is_valid<G: ProbabilisticWitness>(
    game: &G,
    state: &G::ModelState,
    actions: &G::JointActionBuf,
) {
    let mut support = G::SupportBuf::default();
    game.model_step_support(state, actions, &mut support);
    assert!(!support.as_slice().is_empty());
    let mut total_weight = 0u64;
    for outcome in support.as_slice() {
        assert!(outcome.weight > 0);
        total_weight = total_weight.saturating_add(outcome.weight);
    }
    assert!(total_weight > 0);
}
