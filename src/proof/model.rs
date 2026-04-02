//! Proof traits that separate runtime semantics from executable reference models.

use core::fmt::Debug;

use crate::compact::CompactSpec;
use crate::game::Game;
use crate::rng::DeterministicRng;
use crate::types::{PlayerId, Seed, StepOutcome};

/// Safety contracts lifted out of the runtime trait surface.
pub trait SafetyWitness: Game {
    fn safety_state_invariant(&self, state: &Self::State) -> bool {
        self.state_invariant(state)
    }

    fn safety_action_invariant(&self, action: &Self::Action) -> bool {
        self.action_invariant(action)
    }

    fn safety_player_observation_invariant(
        &self,
        state: &Self::State,
        player: PlayerId,
        observation: &Self::Obs,
    ) -> bool {
        self.player_observation_invariant(state, player, observation)
    }

    fn safety_spectator_observation_invariant(
        &self,
        state: &Self::State,
        observation: &Self::Obs,
    ) -> bool {
        self.spectator_observation_invariant(state, observation)
    }

    fn safety_world_view_invariant(&self, state: &Self::State, world: &Self::WorldView) -> bool {
        self.world_view_invariant(state, world)
    }

    fn safety_transition_postcondition(
        &self,
        pre: &Self::State,
        actions: &Self::JointActionBuf,
        post: &Self::State,
        outcome: &StepOutcome<Self::RewardBuf>,
    ) -> bool {
        self.transition_postcondition(pre, actions, post, outcome)
    }
}

impl<T: Game> SafetyWitness for T {}

/// Executable reference semantics for a runtime `Game` implementation.
pub trait ModelGame: Game {
    type ModelState: Clone + Debug + Eq + PartialEq;
    type ModelObs: Clone + Debug + Eq + PartialEq;
    type ModelWorldView: Clone + Debug + Eq + PartialEq;

    fn model_init_with_params(&self, seed: Seed, params: &Self::Params) -> Self::ModelState;
    fn model_is_terminal(&self, state: &Self::ModelState) -> bool;
    fn model_players_to_act(&self, state: &Self::ModelState, out: &mut Self::PlayerBuf);
    fn model_legal_actions(
        &self,
        state: &Self::ModelState,
        player: PlayerId,
        out: &mut Self::ActionBuf,
    );
    fn model_observe_player(&self, state: &Self::ModelState, player: PlayerId) -> Self::ModelObs;
    fn model_observe_spectator(&self, state: &Self::ModelState) -> Self::ModelObs;
    fn model_world_view(&self, state: &Self::ModelState) -> Self::ModelWorldView;
    fn model_step_in_place(
        &self,
        state: &mut Self::ModelState,
        actions: &Self::JointActionBuf,
        rng: &mut DeterministicRng,
        out: &mut StepOutcome<Self::RewardBuf>,
    );

    fn model_compact_spec_for_params(&self, params: &Self::Params) -> CompactSpec {
        self.compact_spec_for_params(params)
    }
}

/// Refinement witness between runtime values and executable model values.
pub trait RefinementWitness: ModelGame + SafetyWitness {
    fn runtime_state_to_model(&self, state: &Self::State) -> Self::ModelState;
    fn runtime_observation_to_model(&self, observation: &Self::Obs) -> Self::ModelObs;
    fn runtime_world_view_to_model(&self, world: &Self::WorldView) -> Self::ModelWorldView;

    fn state_refines_model(&self, state: &Self::State, model: &Self::ModelState) -> bool {
        self.runtime_state_to_model(state) == *model
    }

    fn observation_refines_model(&self, observation: &Self::Obs, model: &Self::ModelObs) -> bool {
        self.runtime_observation_to_model(observation) == *model
    }

    fn world_view_refines_model(
        &self,
        world: &Self::WorldView,
        model: &Self::ModelWorldView,
    ) -> bool {
        self.runtime_world_view_to_model(world) == *model
    }

    fn compact_spec_refines_model(&self, params: &Self::Params) -> bool {
        self.compact_spec_for_params(params) == self.model_compact_spec_for_params(params)
    }
}

/// Explicit marker for games that opt into the stronger proof/refinement surface.
///
/// This is intentionally not blanket-implemented: a game should opt in only after
/// its verification surface and manifest claim are deliberate.
pub trait VerifiedGame: RefinementWitness {}
